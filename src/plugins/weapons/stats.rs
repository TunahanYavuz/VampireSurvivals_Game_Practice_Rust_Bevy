use super::core::{FlameWeapon, LaserWeapon, RayGunWeapon, RocketWeapon, Weapon};
use super::effects::flame_config;
use super::upgrades::{WeaponLevel, WeaponType};
use crate::plugins::common::GameEntity;
use crate::plugins::particle_effects::{ParticleEmitter, SpawnMode};
use crate::plugins::texture_handling::TextureAssets;
use bevy::prelude::*;
#[derive(Component)]
pub struct WeaponStats {
    pub base_damage: f32,
    pub base_fire_rate: f32,
    pub base_speed: f32,
    pub base_range: f32,
    pub damage_growth_per_level: f32,
    pub cooldown_reduction_per_level: f32,
    pub aoe_growth_per_level: f32,
    pub projectile_step_levels: i32,
    pub projectile_bonus_per_step: u32,
}

impl WeaponStats {
    pub fn calculate_damage(&self, level: i32) -> f32 {
        let tiers = (level - 1).max(0) as f32;
        self.base_damage * (1.0 + self.damage_growth_per_level * tiers)
    }
    pub fn calculate_fire_rate(&self, level: i32) -> f32 {
        let tiers = (level - 1).max(0) as f32;
        let reduction = self.cooldown_reduction_per_level * tiers;
        (self.base_fire_rate * (1.0 - reduction)).max(0.05)
    }
    pub fn calculate_speed(&self, level: i32) -> f32 {
        let tiers = (level - 1).max(0) as f32;
        self.base_speed * (1.0 + 0.08 * tiers)
    }
    pub fn calculate_range(&self, level: i32) -> f32 {
        let tiers = (level - 1).max(0) as f32;
        self.base_range * (1.0 + self.aoe_growth_per_level * tiers)
    }
    pub fn projectile_count(&self, level: i32) -> u32 {
        if self.projectile_step_levels <= 0 {
            return 1;
        }
        let tiers = (level - 1).max(0);
        1 + ((tiers / self.projectile_step_levels) as u32 * self.projectile_bonus_per_step)
    }
}

pub fn spawn_weapons_for_player(
    commands: &mut Commands,
    player_entity: Entity,
    _player_pos: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    weapon_name: &str,
    texture_assets: &TextureAssets,
) {
    println!("Spawning weapon for player!");
    match weapon_name {
        "Flame Thrower" => {
            spawn_flame_weapon(
                commands,
                player_entity,
                _player_pos,
                meshes,
                materials,
                texture_assets,
            );
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
    let (damage, fire_rate, speed) = (10.0, 1.0, 130.0);
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage,
            fire_timer: Timer::from_seconds(fire_rate, TimerMode::Repeating),
            speed,
        },
        LaserWeapon {
            color: Color::srgb(0.0, 0.5, 0.0),
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Laser,
        },
        WeaponStats {
            base_damage: damage,
            base_fire_rate: fire_rate,
            base_speed: speed,
            base_range: 0.0,
            damage_growth_per_level: 0.18,
            cooldown_reduction_per_level: 0.06,
            aoe_growth_per_level: 0.0,
            projectile_step_levels: 3,
            projectile_bonus_per_step: 1,
        },
    ));
}

pub fn spawn_rocket_weapon(commands: &mut Commands, player_entity: Entity) {
    let rocket_base_range = 100.0;
    let (damage, fire_rate, speed) = (25.0, 2.0, 100.0);
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage,
            fire_timer: Timer::from_seconds(fire_rate, TimerMode::Repeating),
            speed,
        },
        RocketWeapon {
            explosion_radius: rocket_base_range,
            angle_index: 0,
        },
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::Rocket,
        },
        WeaponStats {
            base_damage: damage,
            base_fire_rate: fire_rate,
            base_speed: speed,
            base_range: rocket_base_range,
            damage_growth_per_level: 0.22,
            cooldown_reduction_per_level: 0.05,
            aoe_growth_per_level: 0.20,
            projectile_step_levels: 2,
            projectile_bonus_per_step: 1,
        },
    ));
}

pub fn spawn_raygun_weapon(commands: &mut Commands, player_entity: Entity) {
    let (damage, fire_rate, speed) = (1.0, 2.0, 0.0);
    commands.spawn((
        GameEntity,
        Weapon {
            owner: player_entity,
            damage,
            fire_timer: Timer::from_seconds(fire_rate, TimerMode::Repeating),
            speed,
        },
        RayGunWeapon::default(),
        WeaponLevel {
            level: 1,
            weapon_type: WeaponType::RayGun,
        },
        WeaponStats {
            base_damage: damage,
            base_fire_rate: fire_rate,
            base_speed: speed,
            base_range: 0.0,
            damage_growth_per_level: 0.20,
            cooldown_reduction_per_level: 0.03,
            aoe_growth_per_level: 0.0,
            projectile_step_levels: 2,
            projectile_bonus_per_step: 1,
        },
    ));
}

pub fn spawn_flame_weapon(
    commands: &mut Commands,
    player_entity: Entity,
    _player_pos: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    texture_assets: &TextureAssets,
) {
    let base_range = 75.0;
    let flame_radius = 80.0;

    commands.entity(player_entity).with_children(|parent| {
        let (damage, fire_rate, speed) = (5.0, 0.1, 0.0);
        parent.spawn((
            Mesh2d(meshes.add(Annulus::new(0.8, 1.0))),
            MeshMaterial2d(
                materials.add(ColorMaterial::from(Color::srgba(0.89, 0.35, 0.13, 0.75))),
            ),
            FlameWeapon { radius: base_range },
            Weapon {
                fire_timer: Timer::from_seconds(fire_rate, TimerMode::Repeating),
                damage,
                owner: player_entity,
                speed,
            },
            WeaponLevel {
                level: 1,
                weapon_type: WeaponType::Flame,
            },
            WeaponStats {
                base_damage: damage,
                base_fire_rate: fire_rate,
                base_speed: speed,
                base_range,
                damage_growth_per_level: 0.16,
                cooldown_reduction_per_level: 0.04,
                aoe_growth_per_level: 0.12,
                projectile_step_levels: 99,
                projectile_bonus_per_step: 0,
            },
            // Flame particle emitter - dairesel spawn
            ParticleEmitter {
                enabled: true,
                spawn_timer: Timer::from_seconds(0.03, TimerMode::Repeating),
                particles_per_spawn: 30,
                config: flame_config(texture_assets),
                offset: Vec3::ZERO,
                spawn_mode: SpawnMode::Circular {
                    radius: flame_radius,
                },
                lifetime: None,
            },
            Transform::default(),
            GlobalTransform::default(),
        ));
    });
}

#[derive(Component)]
pub struct Throwable {
    pub last_direction: Vec3,
}

#[derive(Component)]
pub struct SwordWeapon;

pub fn spawn_throwing_weapon(commands: &mut Commands, player_entity: Entity) {
    commands.spawn((
        GameEntity,
        Throwable {
            last_direction: Vec3::X,
        },
        SwordWeapon,
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
            damage_growth_per_level: 0.17,
            cooldown_reduction_per_level: 0.05,
            aoe_growth_per_level: 0.0,
            projectile_step_levels: 3,
            projectile_bonus_per_step: 1,
        },
    ));
}
