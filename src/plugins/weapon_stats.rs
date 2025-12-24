use bevy::prelude::*;
use crate::plugins::weapon_upgrade::{WeaponLevel, WeaponType};
use crate::plugins::weapons::{GameEntity, LaserWeapon, PlayerAddictedWeapon, RocketWeapon, Weapon}; // GameEntity'yi buradan import edin

#[derive(Component)]
pub struct WeaponStats{
    pub base_damage: f32,
    pub base_fire_rate: f32,
    pub base_speed: f32,
    pub base_range: f32,
}

impl WeaponStats {
    pub fn calculate_damage(&self, level:i32) -> f32{
        match level {
            1 => self.base_damage,
            _ => self.base_damage + ((level -1) as f32 * 10.0)
        }
    }
    pub fn calculate_fire_rate(&self, level:i32) -> f32{
        let bonus = (level-1) as f32 * 0.1;
        (self.base_fire_rate * (1.0 - bonus)).max(0.05)
    }
    pub fn calculate_speed(&self, level:i32) -> f32{
        self.base_speed + ((level - 1) as f32 * 25.0)
    }
    pub fn calculate_range(&self, level:i32) -> f32{
        self.base_range * (1.0 + (level -1) as f32 * 0.15)
    }
}

pub fn spawn_weapons_for_player(
    commands: &mut Commands,
    player_entity: Entity,
    _player_pos: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
){
    println!("Spawning weapons for player!");

    // Lazer silahı
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage: 50.0,
            fire_timer: Timer::from_seconds(0.3, TimerMode::Repeating),
        },
        LaserWeapon {
            speed: 500.0,
            color: Color::srgb(0.0, 1.0, 1.0),
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Laser,
        },
        WeaponStats {
            base_damage: 50.0,
            base_fire_rate: 0.3,
            base_speed: 500.0,
            base_range: 0.0,
        },
    ));

    // Roket silahı
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage: 50.0,
            fire_timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        },
        RocketWeapon {
            speed: 200.0,
            explosion_radius: 100.0,
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Rocket,
        },
        WeaponStats {
            base_damage: 50.0,
            base_fire_rate: 0.2,
            base_speed: 200.0,
            base_range: 0.0,
        },
    ));

    // Alev silahı
    commands.spawn((
        GameEntity,
        Mesh2d(meshes.add(Circle::new(75.0))),
        MeshMaterial2d(materials.add(Color::srgba(1.0, 0.5, 0.0, 0.3))),
        PlayerAddictedWeapon {
            damage: 5.0,
        },
        Weapon {
            fire_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            damage: 5.0,
            owner: player_entity,
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Flame,
        },
        WeaponStats {
            base_damage: 5.0,
            base_fire_rate: 0.1,
            base_speed: 0.0,
            base_range: 75.0,
        },
        Transform::from_translation(_player_pos),
    ));
}