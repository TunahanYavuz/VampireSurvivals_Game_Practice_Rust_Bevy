use crate::plugins::common::GameEntity;
use super::upgrades::{WeaponLevel, WeaponType};
use super::core::{FlameWeapon, LaserWeapon, RayGunWeapon, RocketWeapon, Weapon};
use bevy::prelude::*;
use crate::plugins::particle_effects::{ParticleEmitter, SpawnMode};
use crate::plugins::texture_handling::TextureAssets;
use super::effects::flame_config;
#[derive(Component)]
pub struct WeaponStats {
    pub base_damage: f32,
    pub base_fire_rate: f32,
    pub base_speed: f32,
    pub base_range: f32,
}

impl WeaponStats {
    pub fn calculate_damage(&self, level: i32) -> f32 {
        match level {
            1 => self.base_damage,
            _ => self.base_damage + ((level - 1) as f32 * 10.0),
        }
    }
    pub fn calculate_fire_rate(&self, level: i32) -> f32 {
        let bonus = (level - 1) as f32 * 0.1;
        (self.base_fire_rate * (1.0 - bonus)).max(0.05)
    }
    pub fn calculate_speed(&self, level: i32) -> f32 {
        self.base_speed + ((level - 1) as f32 * 25.0)
    }
    pub fn calculate_range(&self, level: i32) -> f32 {
        self.base_range * (1.0 + (level - 1) as f32 * 0.15)
    }
}

pub fn spawn_weapons_for_player(
    commands: &mut Commands,
    player_entity: Entity,
    _player_pos: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    weapon_name: &str,
    texture_assets: &TextureAssets
) {
    println!("Spawning weapon for player!");
    match weapon_name {
        "Flame Thrower" => {
            spawn_flame_weapon(commands, player_entity, _player_pos, meshes, materials, texture_assets);
        }
        "Laser Gun" => {
            spawn_lazer_weapon(commands, player_entity);
        }
        "Rocket Launcher" => {
            spawn_rocket_weapon(commands, player_entity);
        }
        "Ray Gun" => {
            spawn_raygun_weapon(commands, player_entity);
        }
        "Sword" => {
            spawn_throwing_weapon(commands, player_entity);
        }
        _ => {
            // Default weapon
            spawn_rocket_weapon(commands, player_entity);
        }
    }
}

pub fn spawn_lazer_weapon(commands: &mut Commands, player_entity: Entity) {
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage: 50.0,
            fire_timer: Timer::from_seconds(0.3, TimerMode::Repeating),
            speed: 200.0,
        },
        LaserWeapon {
            color: Color::srgb(0.0, 0.5, 0.0),
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Laser,
        },
        WeaponStats {
            base_damage: 50.0,
            base_fire_rate: 0.3,
            base_speed: 200.0,
            base_range: 0.0,
        },
    ));
}

pub fn spawn_rocket_weapon(commands: &mut Commands, player_entity: Entity) {
    let rocket_base_range = 100.0;
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage: 100.0,
            fire_timer: Timer::from_seconds(0.2, TimerMode::Repeating),
            speed: 200.0,
        },
        RocketWeapon {
            explosion_radius: rocket_base_range,
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Rocket,
        },
        WeaponStats {
            base_damage: 50.0,
            base_fire_rate: 0.2,
            base_speed: 200.0,
            base_range: rocket_base_range,
        },
    ));
}

pub fn spawn_raygun_weapon(commands: &mut Commands, player_entity: Entity) {
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage: 1.0,
            fire_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            speed: 0.0,
        },
        RayGunWeapon::default(),
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::RayGun,
        },
        WeaponStats {
            base_damage: 1.0,
            base_fire_rate: 0.1,
            base_speed: 0.0,
            base_range: 0.0,
        },
    ));
}

pub fn spawn_flame_weapon(
    commands: &mut Commands,
    player_entity: Entity,
    _player_pos: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    texture_assets: &TextureAssets
) {
    let base_range = 75.0;
    let flame_radius = 80.0;

    commands.entity(player_entity).with_children(|parent|{
        parent.spawn((
            Mesh2d(meshes.add(Annulus::new(0.8, 1.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(0.89, 0.35, 0.13, 0.75)))),
            FlameWeapon { radius: base_range },
            Weapon {
                fire_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
                damage: 5.0,
                owner: player_entity,
                speed: 0.0,
            },
            WeaponLevel {
                level: 1,
                weapon_type: WeaponType::Flame,
            },
            WeaponStats {
                base_damage: 5.0,
                base_fire_rate: 0.1,
                base_speed: 0.0,
                base_range: base_range,
            },
            // Flame particle emitter - dairesel spawn
            ParticleEmitter {
                enabled: true,
                spawn_timer: Timer::from_seconds(0.03, TimerMode::Repeating),
                particles_per_spawn: 30,
                config: flame_config(texture_assets),
                offset: Vec3::ZERO,
                spawn_mode: SpawnMode::Circular { radius: flame_radius },
                lifetime: None,
            },
            Transform::default(),
            GlobalTransform::default(),
        ));
    });

}

#[derive(Component)]
pub struct Throwable{
    pub last_direction: Vec3,
}

#[derive(Component)]
pub struct SwordWeapon;

pub fn spawn_throwing_weapon(commands: &mut Commands, player_entity: Entity) {
    commands.spawn((
        GameEntity,
        Throwable{
            last_direction: Vec3::X,
        },
        SwordWeapon ,
        Weapon {
            owner: player_entity,
            damage: 75.0,
            fire_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            speed: 150.0,
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Sword,
        },
        WeaponStats {
            base_damage: 75.0,
            base_fire_rate: 0.5,
            base_speed: 150.0,
            base_range: 0.0,
        },
    ));
}
