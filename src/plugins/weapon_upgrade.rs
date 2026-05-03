use crate::plugins::audio::GameAudioEntity;
use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
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
                )
                    .run_if(in_state(GameState::UpgradeSelection)),
            );
    }
}

/// Silah tipi - sadece tip belirteci, veri içermez
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WeaponType {
    Laser,
    Rocket,
    RayGun,
    Addicted,
    Sword,
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

#[derive(Message)]
pub struct LevelUpEvent {
    #[allow(dead_code)]
    pub level: i32,
}

#[derive(Resource, Default)]
pub struct UpgradeChoices {
    pub options: Vec<UpgradeOption>,
    pub waiting_for_choice: bool,
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
pub fn show_upgrade_choices_on_level_up(
    mut level_up_events: MessageReader<LevelUpEvent>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    mut commands: Commands,
    table: Query<Entity, With<WeaponTable>>,
    asset_server: Res<AssetServer>,
    locale: Res<Locale>,
) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    for _ in level_up_events.read() {
        let options = upgrade_choices.generate_random_options(&locale);

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
    player_q: Query<(Entity, &Transform), With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for event in upgrade_events.read() {
        upgrade_choices.waiting_for_choice = false;

        let weapon_exists = weapons
            .iter()
            .any(|(_, level, ..)| level.weapon_type == event.weapon_type);

        if !weapon_exists {
            // Spawn the new weapon for every player.
            for (player_entity, player_transform) in &player_q {
                match event.weapon_type {
                    WeaponType::Laser => spawn_lazer_weapon(&mut commands, player_entity),
                    WeaponType::Rocket => spawn_rocket_weapon(&mut commands, player_entity),
                    WeaponType::RayGun => spawn_raygun_weapon(&mut commands, player_entity),
                    WeaponType::Addicted => spawn_flame_weapon(
                        &mut commands,
                        player_entity,
                        player_transform.translation,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    ),
                    WeaponType::Sword => spawn_throwing_weapon(&mut commands, player_entity),
                }
            }
            next_state.set(GameState::Playing);
            continue;
        }

        for (mut weapon, mut level, stats, laser, rocket, addicted, raygun, sword) in
            weapons.iter_mut()
        {
            // Silah tipini kontrol et
            if level.weapon_type != event.weapon_type {
                continue;
            }

            // Seviye artır
            level.level += 1;
            let new_level = level.level;

            // Ortak güncellemeler
            weapon.damage = stats.calculate_damage(new_level);
            weapon.speed = stats.calculate_speed(new_level);
            let new_fire_rate = stats.calculate_fire_rate(new_level);
            weapon
                .fire_timer
                .set_duration(std::time::Duration::from_secs_f32(new_fire_rate));

            // Silah tipine göre özel güncellemeler
            match event.weapon_type {
                WeaponType::Laser => {
                    if let Some(_laser_weapon) = laser {
                        // Laser-specific updates (e.g. colour change)
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

            next_state.set(GameState::Playing);
            break;
        }
    }
}

pub fn handle_upgrade_input(
    interaction_q: Query<(&Interaction, &UpgradeButton), (Changed<Interaction>, With<Button>)>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
) {
    for (interaction, upgrade_button) in interaction_q.iter() {
        if *interaction == Interaction::Pressed {
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
