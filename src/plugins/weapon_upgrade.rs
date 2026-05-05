use crate::plugins::audio::GameAudioEntity;
use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::network::{
    NetworkRole, NetOutbox, PendingClientUpgradeChoice, PendingUpgradeApplied,
    PendingUpgradeOptions, UpgradeMode, S2C, NetworkedGameState, encode,
};
use crate::plugins::player::Player;
use crate::plugins::weapon_stats::{
    SwordWeapon, WeaponStats, spawn_flame_weapon, spawn_lazer_weapon, spawn_raygun_weapon,
    spawn_rocket_weapon, spawn_throwing_weapon,
};
use crate::plugins::weapons::{
    LaserWeapon, PlayerAddictedWeapon, RayGunWeapon, RocketWeapon, Weapon,
};
use bevy::prelude::*;
use bevy::ui::Val::Auto;
use rand::prelude::IndexedRandom;
use rand::rng;
use crate::plugins::particle_effects::{ParticleEmitter, SpawnMode};
use serde::{Deserialize, Serialize};

pub struct UpgradePlugin;

impl Plugin for UpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UpgradeChoices>()
            .add_message::<LevelUpEvent>()
            .add_message::<UpgradeSelectedEvent>()
            .add_systems(OnEnter(GameState::UpgradeSelection), create_table_ui)
            .add_systems(
                OnExit(GameState::UpgradeSelection),
                cleanup_upgrade_ui_on_choice,
            )
            .add_systems(
                Update,
                (
                    show_upgrade_choices_on_level_up,
                    handle_upgrade_input,
                    apply_weapon_upgrade,
                    // Network: client receives upgrade options / applied messages.
                    receive_net_upgrade_options,
                    receive_net_upgrade_applied,
                    // Network: host receives upgrade choice from client (Mode A P2).
                    receive_client_upgrade_choice,
                )
                    .run_if(in_state(GameState::UpgradeSelection)),
            );
    }
}

// ─────────────────────────── Types ───────────────────────────────────────

/// Identifies a weapon class — used for upgrade selection and network messages.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WeaponType {
    Laser,
    Rocket,
    RayGun,
    Addicted,
    Sword,
}

impl WeaponType {
    /// Encode as a `u8` for network transmission.
    pub fn to_u8(self) -> u8 {
        match self {
            WeaponType::Laser => 0,
            WeaponType::Rocket => 1,
            WeaponType::RayGun => 2,
            WeaponType::Addicted => 3,
            WeaponType::Sword => 4,
        }
    }

    /// Decode from a `u8` received over the network.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(WeaponType::Laser),
            1 => Some(WeaponType::Rocket),
            2 => Some(WeaponType::RayGun),
            3 => Some(WeaponType::Addicted),
            4 => Some(WeaponType::Sword),
            _ => None,
        }
    }
}

#[derive(Component, Clone)]
pub struct UpgradeOption {
    pub weapon_type: WeaponType,
    pub name: String,
    pub description: String,
    #[allow(dead_code)]
    pub icon: Option<Handle<Image>>,
}

#[derive(Message)]
pub struct UpgradeSelectedEvent {
    pub weapon_type: WeaponType,
}

/// Fired when a player levels up.
#[derive(Message)]
pub struct LevelUpEvent {
    #[allow(dead_code)]
    pub level: i32,
    /// Which player levelled up (0 = P1, 1 = P2).
    pub player_index: u8,
}

#[derive(Resource, Default)]
pub struct UpgradeChoices {
    pub options: Vec<UpgradeOption>,
    pub waiting_for_choice: bool,
    /// Which player this upgrade applies to.
    /// `None` means all players (Mode B).  `Some(idx)` means only that player.
    pub for_player: Option<u8>,
}

#[derive(Component)]
pub struct WeaponLevel {
    pub level: i32,
    pub weapon_type: WeaponType,
}

impl UpgradeChoices {
    pub fn generate_random_options(&mut self, locale: &Locale) -> Vec<UpgradeOption> {
        let all_options = vec![
            UpgradeOption {
                weapon_type: WeaponType::Laser,
                name: locale.t("upgrade_laser").to_string(),
                description: locale.t("upgrade_laser_desc").to_string(),
                icon: None,
            },
            UpgradeOption {
                weapon_type: WeaponType::Rocket,
                name: locale.t("upgrade_rocket").to_string(),
                description: locale.t("upgrade_rocket_desc").to_string(),
                icon: None,
            },
            UpgradeOption {
                weapon_type: WeaponType::Addicted,
                name: locale.t("upgrade_flame").to_string(),
                description: locale.t("upgrade_flame_desc").to_string(),
                icon: None,
            },
            UpgradeOption {
                weapon_type: WeaponType::RayGun,
                name: locale.t("upgrade_raygun").to_string(),
                description: locale.t("upgrade_raygun_desc").to_string(),
                icon: None,
            },
            UpgradeOption {
                weapon_type: WeaponType::Sword,
                name: locale.t("upgrade_sword").to_string(),
                description: locale.t("upgrade_sword_desc").to_string(),
                icon: None,
            },
        ];
        let mut rng = rng();
        let selected: Vec<_> = all_options.choose_multiple(&mut rng, 3).cloned().collect();
        self.options = selected.clone();
        self.waiting_for_choice = true;
        selected
    }
}

#[derive(Component)]
pub struct WeaponTable;

#[derive(Component)]
pub struct UpgradeButton(pub WeaponType);

// ─────────────────────────── Systems ─────────────────────────────────────

pub fn show_upgrade_choices_on_level_up(
    mut level_up_events: MessageReader<LevelUpEvent>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    mut commands: Commands,
    table: Query<Entity, With<WeaponTable>>,
    asset_server: Res<AssetServer>,
    locale: Res<Locale>,
    role: Res<NetworkRole>,
    upgrade_mode: Res<UpgradeMode>,
    outbox: Option<Res<NetOutbox>>,
) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    for event in level_up_events.read() {
        let options = upgrade_choices.generate_random_options(&locale);

        // Determine which player's weapons this upgrade targets.
        let for_player: Option<u8> = match *upgrade_mode {
            // Mode B: shared — applies to all players.
            UpgradeMode::Shared => None,
            // Mode A: independent — applies to only the levelling player.
            UpgradeMode::Independent => Some(event.player_index),
        };
        upgrade_choices.for_player = for_player;

        // Network: if host, broadcast options + state change to client.
        if *role == NetworkRole::Host {
            if let Some(ref outbox) = outbox {
                let opts: Vec<u8> = options.iter().map(|o| o.weapon_type.to_u8()).collect();
                let fp = for_player.unwrap_or(255);

                // In Mode A, only send options to client if it's P2's turn.
                // In Mode B, always send.
                let should_notify_client = match *upgrade_mode {
                    UpgradeMode::Shared => true,
                    UpgradeMode::Independent => event.player_index == 1,
                };

                if should_notify_client {
                    if let Ok(frame) = encode(&S2C::UpgradeOptions { opts, for_player: fp }) {
                        let _ = outbox.0.send(frame);
                    }
                }

                // In Mode B, pause the client too.
                if *upgrade_mode == UpgradeMode::Shared {
                    if let Ok(frame) =
                        encode(&S2C::StateChange(NetworkedGameState::UpgradeSelection))
                    {
                        let _ = outbox.0.send(frame);
                    }
                }
            }
        }

        // Build the UI table.
        let Ok(table_entity) = table.single() else {
            commands.spawn((WeaponTable, Node::default()));
            continue;
        };
        let options_len = options.len() as f32;
        for (i, option) in options.iter().enumerate() {
            commands.entity(table_entity).with_children(|parent| {
                parent.spawn((
                    Button::default(),
                    UpgradeButton(option.weapon_type),
                    Text::new(format!(
                        "{} {} {} - {}",
                        locale.t("option_prefix"),
                        i + 1,
                        option.name,
                        option.description
                    )),
                    TextFont {
                        font: font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    Node {
                        height: Val::Percent(100.0 / options_len),
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    Outline {
                        width: Val::Px(2.0),
                        offset: Val::Px(0.0),
                        color: Color::srgba(0.0, 0.1, 0.2, 0.8),
                    },
                )).with_children(|parent| {
                    if let Some(icon_handle) = &option.icon {
                        parent.spawn((
                            ImageNode::new(icon_handle.clone()),
                            Node{
                                left: Val::Percent(75.0),
                                top: Val::Percent(45.0),
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                margin: UiRect::right(Val::Px(10.0)),
                                border_radius: BorderRadius::all(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                            Outline{
                                width: Val::Px(1.0),
                                offset: Val::Px(0.0),
                                color: Color::srgba(0.0, 0.0, 0.0, 0.8),
                            },

                        ));
                    }
                });
            });
        }
    }
}

pub fn create_table_ui(mut commands: Commands) {
    commands.spawn((
        WeaponTable,
        Node {
            width: Val::Percent(40.0),
            height: Val::Percent(50.0),
            margin: UiRect {
                left: Auto,
                right: Auto,
                top: Auto,
                bottom: Auto,
            },
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        BackgroundColor(Color::srgba(0.7137, 0.7137, 0.7137, 0.92)),
    ));
}

/// Apply a chosen upgrade to the appropriate player(s) and return to Playing.
///
/// `for_player`:
/// - `None` → applies to ALL players' weapons (Mode B).
/// - `Some(idx)` → applies only to weapons owned by the player with `player_index == idx`.
pub fn apply_weapon_upgrade(
    mut upgrade_events: MessageReader<UpgradeSelectedEvent>,
    mut weapons: Query<(
        &mut Weapon,
        &mut WeaponLevel,
        &WeaponStats,
        Option<&mut LaserWeapon>,
        Option<&mut RocketWeapon>,
        Option<(&mut PlayerAddictedWeapon, &mut ParticleEmitter)>,
        Option<&mut RayGunWeapon>,
        Option<&mut SwordWeapon>,
    )>,
    mut next_state: ResMut<NextState<GameState>>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    mut commands: Commands,
    player_q: Query<(Entity, &Transform, &Player), With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
    upgrade_mode: Res<UpgradeMode>,
) {
    for event in upgrade_events.read() {
        upgrade_choices.waiting_for_choice = false;
        let for_player = upgrade_choices.for_player;

        // Determine which player entities are eligible.
        let eligible_players: Vec<(Entity, Vec3)> = player_q
            .iter()
            .filter(|(_, _, p)| {
                for_player.is_none() || for_player == Some(p.player_index)
            })
            .map(|(e, t, _)| (e, t.translation))
            .collect();

        let weapon_exists = weapons.iter().any(|(_, level, ..)| {
            level.weapon_type == event.weapon_type
                && eligible_players.iter().any(|(pe, _)| {
                    // Check if this weapon belongs to an eligible player.
                    // Weapons store their owner entity in the Weapon component.
                    weapons
                        .iter()
                        .find(|(w, l, ..)| l.weapon_type == event.weapon_type && w.owner == *pe)
                        .is_some()
                })
        });

        if !weapon_exists {
            // Spawn the weapon for eligible players only.
            for (player_entity, player_pos) in &eligible_players {
                match event.weapon_type {
                    WeaponType::Laser => spawn_lazer_weapon(&mut commands, *player_entity),
                    WeaponType::Rocket => spawn_rocket_weapon(&mut commands, *player_entity),
                    WeaponType::RayGun => spawn_raygun_weapon(&mut commands, *player_entity),
                    WeaponType::Addicted => spawn_flame_weapon(
                        &mut commands,
                        *player_entity,
                        *player_pos,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    ),
                    WeaponType::Sword => spawn_throwing_weapon(&mut commands, *player_entity),
                }
            }
        } else {
            for (mut weapon, mut level, stats, laser, rocket, addicted, raygun, sword) in
                weapons.iter_mut()
            {
                if level.weapon_type != event.weapon_type {
                    continue;
                }
                // Skip weapons that don't belong to an eligible player.
                if !eligible_players.iter().any(|(pe, _)| weapon.owner == *pe) {
                    continue;
                }

                level.level += 1;
                let new_level = level.level;

                weapon.damage = stats.calculate_damage(new_level);
                weapon.speed = stats.calculate_speed(new_level);
                let new_fire_rate = stats.calculate_fire_rate(new_level);
                weapon
                    .fire_timer
                    .set_duration(std::time::Duration::from_secs_f32(new_fire_rate));

                match event.weapon_type {
                    WeaponType::Laser => {
                        if let Some(_laser_weapon) = laser {
                            // Laser-specific updates
                        }
                    }
                    WeaponType::Rocket => {
                        if let Some(mut rocket_weapon) = rocket {
                            rocket_weapon.explosion_radius = stats.calculate_range(new_level);
                        }
                    }
                    WeaponType::Addicted => {
                        if let Some((mut addicted_weapon, mut emitter)) = addicted {
                            addicted_weapon.radius = stats.calculate_range(new_level);
                            if let SpawnMode::Circular { ref mut radius } = emitter.spawn_mode {
                                *radius = addicted_weapon.radius;
                            }
                        }
                    }
                    WeaponType::RayGun => {
                        if let Some(mut raygun) = raygun {
                            raygun.pierce_count += 1;
                        }
                    }
                    WeaponType::Sword => {
                        if let Some(_sword) = sword {}
                    }
                }
            }
        }

        // Broadcast the applied upgrade to the client (host only).
        if *role == NetworkRole::Host {
            if let Some(ref outbox) = outbox {
                let fp = for_player.unwrap_or(255);
                if let Ok(frame) = encode(&S2C::UpgradeApplied {
                    weapon_type: event.weapon_type.to_u8(),
                    for_player: fp,
                }) {
                    let _ = outbox.0.send(frame);
                }

                // In Mode B, tell the client to resume Playing.
                if *upgrade_mode == UpgradeMode::Shared {
                    if let Ok(frame) = encode(&S2C::StateChange(NetworkedGameState::Playing)) {
                        let _ = outbox.0.send(frame);
                    }
                }
            }
        }

        next_state.set(GameState::Playing);
    }
}

pub fn handle_upgrade_input(
    interaction_q: Query<(&Interaction, &UpgradeButton), (Changed<Interaction>, With<Button>)>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
    role: Res<NetworkRole>,
    upgrade_choices: Res<UpgradeChoices>,
    outbox: Option<Res<NetOutbox>>,
) {
    for (interaction, upgrade_button) in interaction_q.iter() {
        if *interaction == Interaction::Pressed {
            // Client: in Mode A (P2's upgrade), send the choice to the host
            // instead of applying locally.  The host will broadcast UpgradeApplied.
            if *role == NetworkRole::Client {
                if let (Some( outbox), Some(1)) = (&outbox, upgrade_choices.for_player) {
                    use crate::plugins::network::C2S;
                    if let Ok(frame) =
                        encode(&C2S::UpgradeChosen(upgrade_button.0.to_u8()))
                    {
                        let _ = outbox.0.send(frame);
                    }
                    // Client returns to Playing after sending choice.
                    upgrade_events.write(UpgradeSelectedEvent {
                        weapon_type: upgrade_button.0,
                    });
                    return;
                }
            }

            upgrade_events.write(UpgradeSelectedEvent {
                weapon_type: upgrade_button.0,
            });
        }
    }
}

pub fn cleanup_upgrade_ui_on_choice(
    table: Query<Entity, With<WeaponTable>>,
    audio_entity: Query<Entity, With<GameAudioEntity>>,
    mut commands: Commands,
) {
    for table_entity in table.iter() {
        commands.entity(table_entity).try_despawn();
    }
    for audio_entity in audio_entity.iter() {
        commands.entity(audio_entity).despawn();
    }
}

// ─────────────────────────── Network receive systems ─────────────────────

/// Client: receive `UpgradeOptions` from the host and populate the UI.
fn receive_net_upgrade_options(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingUpgradeOptions>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    locale: Res<Locale>,
    mut commands: Commands,
    table: Query<Entity, With<WeaponTable>>,
    asset_server: Res<AssetServer>,
) {
    if *role != NetworkRole::Client {
        return;
    }
    let Some((opts_u8, for_player)) = pending.0.take() else {
        return;
    };

    let for_player_opt = if for_player == 255 {
        None
    } else {
        Some(for_player)
    };
    upgrade_choices.for_player = for_player_opt;

    // Convert u8 indices back to UpgradeOption.
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    let options: Vec<UpgradeOption> = opts_u8
        .iter()
        .filter_map(|&idx| WeaponType::from_u8(idx))
        .map(|wt| UpgradeOption {
            weapon_type: wt,
            name: upgrade_option_name(wt, &locale).to_string(),
            description: upgrade_option_desc(wt, &locale).to_string(),
            icon: None,
        })
        .collect();

    upgrade_choices.options = options.clone();
    upgrade_choices.waiting_for_choice = true;

    // Populate the UI table.
    let Ok(table_entity) = table.single() else {
        return;
    };
    let options_len = options.len() as f32;
    for (i, option) in options.iter().enumerate() {
        commands.entity(table_entity).with_children(|parent| {
            parent.spawn((
                Button::default(),
                UpgradeButton(option.weapon_type),
                Text::new(format!(
                    "Option {} {} - {}",
                    i + 1,
                    option.name,
                    option.description
                )),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                Node {
                    height: Val::Percent(100.0 / options_len),
                    width: Val::Percent(100.0),
                    ..default()
                },
                Outline {
                    width: Val::Px(2.0),
                    offset: Val::Px(0.0),
                    color: Color::srgba(0.0, 0.1, 0.2, 0.8),
                },
            ));
        });
    }
}

/// Client: apply an `UpgradeApplied` message received from the host.
fn receive_net_upgrade_applied(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingUpgradeApplied>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
) {
    if *role != NetworkRole::Client {
        return;
    }
    let Some((weapon_idx, _for_player)) = pending.0.take() else {
        return;
    };
    if let Some(wt) = WeaponType::from_u8(weapon_idx) {
        upgrade_events.write(UpgradeSelectedEvent { weapon_type: wt });
    }
}

/// Host: receive the client's P2 upgrade choice (Mode A) and apply it.
fn receive_client_upgrade_choice(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingClientUpgradeChoice>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
) {
    if *role != NetworkRole::Host {
        return;
    }
    let Some(weapon_idx) = pending.0.take() else {
        return;
    };
    if let Some(wt) = WeaponType::from_u8(weapon_idx) {
        // Apply to P2 only.
        upgrade_choices.for_player = Some(1);
        upgrade_events.write(UpgradeSelectedEvent { weapon_type: wt });
    }
}

// ────────────────────────── Helpers ──────────────────────────────────────

fn upgrade_option_name(wt: WeaponType, locale: &Locale) -> &str {
    match wt {
        WeaponType::Laser => locale.t("upgrade_laser"),
        WeaponType::Rocket => locale.t("upgrade_rocket"),
        WeaponType::Addicted => locale.t("upgrade_flame"),
        WeaponType::RayGun => locale.t("upgrade_raygun"),
        WeaponType::Sword => locale.t("upgrade_sword"),
    }
}

fn upgrade_option_desc(wt: WeaponType, locale: &Locale) -> &str {
    match wt {
        WeaponType::Laser => locale.t("upgrade_laser_desc"),
        WeaponType::Rocket => locale.t("upgrade_rocket_desc"),
        WeaponType::Addicted => locale.t("upgrade_flame_desc"),
        WeaponType::RayGun => locale.t("upgrade_raygun_desc"),
        WeaponType::Sword => locale.t("upgrade_sword_desc"),
    }
}

