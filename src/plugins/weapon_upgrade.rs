use bevy::prelude::*;
use rand::seq::{IndexedRandom,};
use crate::plugins::game_state::GameState;
use crate::plugins::weapon_stats::WeaponStats;
use crate::plugins::weapons::{LaserWeapon, RocketWeapon, Weapon};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponType{
    Laser,
    Rocket,
    Flame,
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
                weapon_type: WeaponType::Laser,
                name : "Laser silahı Güçlendir".to_string(),
                description: "Hasar +10, Hız +%5".to_string(),
                icon: None
            },
            UpgradeOption {
                weapon_type: WeaponType::Rocket,
                name: "Roket Silahı Güçlendir".to_string(),
                description: "Hasar +15, Patlama +10".to_string(),
                icon: None,
            },
            UpgradeOption {
                weapon_type: WeaponType::Flame,
                name: "Alev Silahı Güçlendir".to_string(),
                description: "Hasar +3, Alan +15%".to_string(),
                icon: None,
            },
        ];
        let mut rng = rand::rng();
        let selected: Vec<_> = all_options.choose_multiple(&mut rng, 3).cloned().collect();
        self.options = selected.clone();
        self.waiting_for_choice = true;
        selected
    }
}



pub fn show_upgrade_choices_on_level_up(
    mut level_up_events: MessageReader<LevelUpEvent>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    mut next_state: ResMut<NextState<GameState>>,
){
    for event in level_up_events.read() {
        println!("Level {}! Seçim yapın:", event.level);

        let options = upgrade_choices.generate_random_options();

        // Oyunu duraklat (yeni state ekleyebilirsiniz)
        next_state.set(GameState::UpgradeSelection);
        // UI göster (şimdilik console)
        for (i, option) in options.iter().enumerate() {
            println!("  [{}] {} - {}", i + 1, option.name, option.description);
        }

        println!("1, 2 veya 3 tuşuna basın...");
    }
}

pub fn apply_weapon_upgrade(
    mut upgrade_events: MessageReader<UpgradeSelectedEvent>,
    mut weapons: Query<(&mut Weapon, &mut WeaponLevel, &WeaponStats,
    Option<&mut LaserWeapon>, Option<&mut RocketWeapon>, Option<&mut Transform>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
){
    for event in upgrade_events.read() {
        upgrade_choices.waiting_for_choice = false;
        for (mut weapon, mut level,
            stats, laser,
            rocket,
            transform) in weapons.iter_mut() {
            if level.weapon_type != event.weapon_type{
                continue;
            }
            level.level += 1;
            let new_level = level.level;
            weapon.damage = stats.calculate_damage(new_level);
            let new_fire_rate = stats.calculate_fire_rate(new_level);
            weapon.fire_timer.set_duration(std::time::Duration::from_secs_f32(new_fire_rate));

            match event.weapon_type {
                WeaponType::Laser => {
                    if let Some(mut laser) = laser {
                        laser.speed = stats.calculate_speed(new_level);
                        println!("✨ Laser Level {}! Damage: {}, Speed: {}",
                                 new_level, weapon.damage, laser.speed);
                    }
                }
                WeaponType::Rocket => {
                    if let Some(mut rocket) = rocket {
                        rocket.speed = stats.calculate_speed(new_level);
                        rocket.explosion_radius = 100.0 + ((new_level - 1) as f32 * 10.0);
                        println!("✨ Rocket Level {}! Damage: {}, Explosion: {}",
                                 new_level, weapon.damage, rocket.explosion_radius);
                    }
                }
                WeaponType::Flame => {
                    if let Some(mut trans) = transform {
                        let scale = stats.calculate_range(new_level) / stats.base_range;
                        trans.scale = Vec3::splat(scale);
                        println!("✨ Flame Level {}! Damage: {}, Scale: {:.1}x",
                                 new_level, weapon.damage, scale);
                    }
                }
            }
            next_state.set(GameState::Playing);
            break;
        }
    }
}

pub fn handle_upgrade_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    upgrade_choices: Res<UpgradeChoices>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
){
    if !upgrade_choices.waiting_for_choice {
        return;
    }

    let selected_weapon = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(0)
    }else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(1)
    }else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(2)
    }else { None };

    if let Some(index) = selected_weapon {
        if let Some(option) = upgrade_choices.options.get(index) {
            upgrade_events.write(UpgradeSelectedEvent{
                weapon_type: option.weapon_type
            });
        }
    }
}