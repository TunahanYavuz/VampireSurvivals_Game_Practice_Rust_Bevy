use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::network::{
    NetOutbox, NetworkRole, NetworkedGameState, S2C, UpgradeMode, encode,
};
use crate::plugins::particle_effects::{ParticleEmitter, SpawnMode};
use crate::plugins::player::Player;
use crate::plugins::texture_handling::TextureAssets;
use bevy::prelude::*;
use rand::prelude::IndexedRandom;
use rand::rng;
use serde::{Deserialize, Serialize};

use super::core::{FlameWeapon, LaserWeapon, RayGunWeapon, RocketWeapon, Weapon};
use super::stats::{
    SwordWeapon, WeaponStats, spawn_flame_weapon, spawn_lazer_weapon, spawn_raygun_weapon,
    spawn_rocket_weapon, spawn_throwing_weapon,
};
use super::upgrade_screen::{
    cleanup_upgrade_ui_on_choice, enter_host_upgrade_ui, enter_remote_upgrade_ui,
    handle_upgrade_input, show_upgrade_choices_on_level_up,
};
use super::upgrades_net::{
    receive_client_upgrade_choice, receive_net_upgrade_applied, receive_net_upgrade_options,
};

pub struct UpgradePlugin;

impl Plugin for UpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UpgradeChoices>()
            .add_message::<LevelUpEvent>()
            .add_message::<UpgradeSelectedEvent>()
            .add_systems(OnEnter(GameState::HostUpgrade), enter_host_upgrade_ui)
            .add_systems(OnEnter(GameState::RemoteUpgrade), enter_remote_upgrade_ui)
            .add_systems(Update, show_upgrade_choices_on_level_up)
            .add_systems(Update, receive_net_upgrade_options)
            .add_systems(OnExit(GameState::HostUpgrade), cleanup_upgrade_ui_on_choice)
            .add_systems(
                OnExit(GameState::RemoteUpgrade),
                cleanup_upgrade_ui_on_choice,
            )
            .add_systems(
                Update,
                handle_upgrade_input.run_if(
                    in_state(GameState::HostUpgrade).or(in_state(GameState::RemoteUpgrade)),
                ),
            )
            .add_systems(
                Update,
                (
                    apply_weapon_upgrade,
                    receive_net_upgrade_applied,
                    receive_client_upgrade_choice,
                )
                    .run_if(
                        in_state(GameState::HostUpgrade).or(in_state(GameState::RemoteUpgrade)),
                    ),
            );
    }
}

// ─────────────────────────── Types ───────────────────────────────────────

/// Identifies a weapon class for upgrades and network messages.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WeaponType {
    Laser,
    Rocket,
    RayGun,
    Flame,
    Sword,
}

impl WeaponType {
    pub fn to_u8(self) -> u8 {
        match self {
            WeaponType::Laser => 0,
            WeaponType::Rocket => 1,
            WeaponType::RayGun => 2,
            WeaponType::Flame => 3,
            WeaponType::Sword => 4,
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(WeaponType::Laser),
            1 => Some(WeaponType::Rocket),
            2 => Some(WeaponType::RayGun),
            3 => Some(WeaponType::Flame),
            4 => Some(WeaponType::Sword),
            _ => None,
        }
    }
}

#[derive(Component, Clone, Debug)]
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
    pub player_index: u8,
}

#[derive(Resource, Default)]
pub struct UpgradeChoices {
    pub options: Vec<UpgradeOption>,
    pub waiting_for_choice: bool,
    pub for_player: Option<u8>,
}

#[derive(Component)]
pub struct WeaponLevel {
    pub level: i32,
    pub weapon_type: WeaponType,
}

#[derive(Component)]
pub struct WeaponTable;

#[derive(Component)]
pub struct UpgradeButton(pub WeaponType);

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
                weapon_type: WeaponType::Flame,
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

// ─────────────────────────── Upgrade Logic ───────────────────────────────

pub fn apply_weapon_upgrade(
    mut upgrade_events: MessageReader<UpgradeSelectedEvent>,
    mut weapons: Query<(
        &mut Weapon,
        &mut WeaponLevel,
        &WeaponStats,
        Option<&mut LaserWeapon>,
        Option<&mut RocketWeapon>,
        Option<(&mut FlameWeapon, &mut ParticleEmitter)>,
        Option<&mut RayGunWeapon>,
        Option<&mut SwordWeapon>,
    )>,
    mut next_state: ResMut<NextState<GameState>>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    mut commands: Commands,
    player_q: Query<(Entity, &Transform, &Player), With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    texture_assets: Res<TextureAssets>,
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
    upgrade_mode: Res<UpgradeMode>,
) {
    for event in upgrade_events.read() {
        upgrade_choices.waiting_for_choice = false;
        let for_player = upgrade_choices.for_player;

        let eligible_players: Vec<(Entity, Vec3)> = player_q
            .iter()
            .filter(|(_, _, p)| for_player.is_none() || for_player == Some(p.player_index))
            .map(|(e, t, _)| (e, t.translation))
            .collect();

        let weapon_exists = weapons.iter().any(|(_, level, ..)| {
            level.weapon_type == event.weapon_type
                && eligible_players.iter().any(|(pe, _)| {
                    weapons
                        .iter()
                        .find(|(w, l, ..)| l.weapon_type == event.weapon_type && w.owner == *pe)
                        .is_some()
                })
        });

        if !weapon_exists {
            for (player_entity, player_pos) in &eligible_players {
                match event.weapon_type {
                    WeaponType::Laser => spawn_lazer_weapon(&mut commands, *player_entity),
                    WeaponType::Rocket => spawn_rocket_weapon(&mut commands, *player_entity),
                    WeaponType::RayGun => spawn_raygun_weapon(&mut commands, *player_entity),
                    WeaponType::Flame => spawn_flame_weapon(
                        &mut commands,
                        *player_entity,
                        *player_pos,
                        &mut meshes,
                        &mut materials,
                        &texture_assets,
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
                    WeaponType::Flame => {
                        if let Some((mut addicted_weapon, mut emitter)) = addicted {
                            addicted_weapon.radius = stats.calculate_range(new_level);
                            if let SpawnMode::Circular { ref mut radius } = emitter.spawn_mode {
                                *radius = addicted_weapon.radius;
                            }
                        }
                    }
                    WeaponType::RayGun => {
                        if let Some(mut raygun) = raygun {
                            raygun.pierce_count = 3 + ((new_level - 1).max(0) as u32 / 2);
                        }
                    }
                    WeaponType::Sword => if let Some(_sword) = sword {},
                }
            }
        }

        if *role == NetworkRole::Host {
            if let Some(ref outbox) = outbox {
                let fp = for_player.unwrap_or(255);
                if let Ok(frame) = encode(&S2C::UpgradeApplied {
                    weapon_type: event.weapon_type.to_u8(),
                    for_player: fp,
                }) {
                    let _ = outbox.0.send(frame);
                }

                let should_resume_client = match *upgrade_mode {
                    UpgradeMode::Shared => true,
                    UpgradeMode::Independent => for_player.is_some(),
                };
                if should_resume_client {
                    if let Ok(frame) = encode(&S2C::StateChange(NetworkedGameState::Playing)) {
                        let _ = outbox.0.send(frame);
                    }
                }
            }
        }

        if *role == NetworkRole::Host || *role == NetworkRole::Solo {
            next_state.set(GameState::Playing);
        }
    }
}

// ─────────────────────────── Helpers ─────────────────────────────────────

pub(crate) fn upgrade_option_name(wt: WeaponType, locale: &Locale) -> &str {
    match wt {
        WeaponType::Laser => locale.t("upgrade_laser"),
        WeaponType::Rocket => locale.t("upgrade_rocket"),
        WeaponType::Flame => locale.t("upgrade_flame"),
        WeaponType::RayGun => locale.t("upgrade_raygun"),
        WeaponType::Sword => locale.t("upgrade_sword"),
    }
}

pub(crate) fn upgrade_option_desc(wt: WeaponType, locale: &Locale) -> &str {
    match wt {
        WeaponType::Laser => locale.t("upgrade_laser_desc"),
        WeaponType::Rocket => locale.t("upgrade_rocket_desc"),
        WeaponType::Flame => locale.t("upgrade_flame_desc"),
        WeaponType::RayGun => locale.t("upgrade_raygun_desc"),
        WeaponType::Sword => locale.t("upgrade_sword_desc"),
    }
}
