use bevy::prelude::*;
use bevy::ui::Val::Auto;
use rand::{rng};
use rand::prelude::IndexedRandom;
use crate::plugins::game_state::GameState;
use crate::plugins::weapon_stats::WeaponStats;
use crate::plugins::weapons::{LaserWeapon, PlayerAddictedWeapon, Projectile, ProjectileKind, RocketWeapon, Weapon};

#[derive(Clone, Copy)]
#[derive(PartialEq)]
pub enum WeaponType{
    Projectile{weapon: ProjectileKind},
    Addicted,
}

#[derive(Component,Clone)]
pub struct UpgradeOption{
    pub weapon_type: WeaponType,
    pub name: String,
    pub description: String,
    pub icon: Option<Handle<Image>>,
}

#[derive(Message)]
pub struct UpgradeSelectedEvent{
    pub weapon_type: WeaponType
}

#[derive(Message)]
pub struct LevelUpEvent{
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
    pub fn generate_random_options(&mut self) -> Vec<UpgradeOption>{

        let all_options = vec![
            UpgradeOption{
                weapon_type: WeaponType::Projectile {weapon: ProjectileKind::Laser{lazer_weapon: LaserWeapon{color: Color::srgba(1.0, 0.0, 0.0, 1.0)}}},
                name : "Laser silahı Güçlendir".to_string(),
                description: "Hasar +10, Hız +%5".to_string(),
                icon: None
            },
            UpgradeOption {
                weapon_type: WeaponType::Projectile {weapon: ProjectileKind::Rocket{rocket_weapon: RocketWeapon{explosion_radius: 30.0}}},
                name: "Roket Silahı Güçlendir".to_string(),
                description: "Hasar +15, Patlama +10".to_string(),
                icon: None,
            },
            UpgradeOption {
                weapon_type: WeaponType::Addicted,
                name: "Alev Silahı Güçlendir".to_string(),
                description: "Hasar +3, Alan +15%".to_string(),
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
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    table: Query<Entity, With<WeaponTable>>,
    asset_server: Res<AssetServer>,
){
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    for _ in level_up_events.read() {
        let options = upgrade_choices.generate_random_options();

        next_state.set(GameState::UpgradeSelection);

        let Ok(table_entity) = table.single() else {
            commands.spawn((WeaponTable, Node::default()));
            continue;
        };
        let options_len = options.len() as f32;
        for (i ,option) in options.iter().enumerate() {
            commands.entity(table_entity).with_children(|parent| {
                parent.spawn((
                    Button::default(), UpgradeButton(option.weapon_type),
                    Text::new(format!("Seçenek {} {} - {}", i, option.name, option.description)),
                    TextFont{
                        font: font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    Node{height: Val::Percent(100.0/ options_len), width: Val::Percent(100.0), ..default()},
                    Outline{
                        width: Val::Px(2.0),
                        offset: Val::Px(0.0),
                        color: Color::srgba(0.0, 0.1, 0.2, 0.8),
                    }
                ));
            });
        }
    }
}

pub fn create_table_ui(
    mut commands: Commands,
){
    commands.spawn((
        WeaponTable,
        Node{
            width: Val::Percent(40.0),
            height: Val::Percent(50.0),
            margin: UiRect{left: Auto, right: Auto, top: Auto, bottom: Auto},
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        BackgroundColor(Color::srgba(0.7137, 0.7137, 0.7137, 0.92))
    ));
}

pub fn apply_weapon_upgrade(
    mut upgrade_events: MessageReader<UpgradeSelectedEvent>,
    mut weapons: Query<(&mut Weapon, &mut WeaponLevel, &WeaponStats,
    Option<&mut PlayerAddictedWeapon>, Option<&mut Projectile>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
){
    for event in upgrade_events.read() {
        upgrade_choices.waiting_for_choice = false;
        for (mut weapon, mut level,
            stats, addicted_weapon,
            projectile,
            ) in weapons.iter_mut() {
            if std::mem::discriminant(&level.weapon_type) != std::mem::discriminant(&event.weapon_type) {
                continue;
            }
            level.level += 1;
            let new_level = level.level;
            weapon.damage = stats.calculate_damage(new_level);
            let new_fire_rate = stats.calculate_fire_rate(new_level);
            weapon.fire_timer.set_duration(std::time::Duration::from_secs_f32(new_fire_rate));

            match event.weapon_type {
                WeaponType::Projectile { weapon: _ }  => {
                    if let Some(mut projectile) = projectile {
                        match &mut projectile.kind {
                            ProjectileKind::Laser { lazer_weapon: _ } => {
                                projectile.damage = stats.calculate_damage(new_level);
                                println!("Laser");
                            },
                            ProjectileKind::Rocket { .. } => {
                                projectile.damage = stats.calculate_damage(new_level);
                                stats.calculate_range(new_level);
                                println!("Rocket");
                            },
                        }
                    }
                    println!("Projectile silahı yükseltildi! Yeni seviye: {}", new_level);
                },
                WeaponType::Addicted  => {
                    if let Some(mut addicted_weapon) = addicted_weapon {
                        weapon.damage = stats.calculate_damage(new_level);
                        addicted_weapon.radius = stats.calculate_range(new_level);

                    }
                    println!("Bağımlı silah yükseltildi! Yeni seviye: {}", new_level);
                },
            }
            next_state.set(GameState::Playing);
            break;
        }
    }
}

pub fn handle_upgrade_input(
    interaction_q: Query<(&Interaction, &UpgradeButton), (Changed<Interaction>, With<Button>)>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
){
    for (interaction, upgrade_button) in interaction_q.iter() {
        if *interaction == Interaction::Pressed {
            upgrade_events.write(
                UpgradeSelectedEvent{
                    weapon_type: upgrade_button.0,
            });

        }
    }
}

pub fn cleanup_upgrade_ui_on_choice(
    table: Query<Entity, With<WeaponTable>>,
    mut commands: Commands,
){
    for table_entity in table.iter() {
        commands.entity(table_entity).try_despawn();
    }
}