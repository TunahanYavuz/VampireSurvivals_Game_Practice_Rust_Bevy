use super::upgrades::WeaponType;
use crate::plugins::common::GameEntity;
use crate::plugins::game_state::GameState;
use crate::plugins::particle_effects::{
    ParticleConfig, ParticleEmitter, SpawnMode, spawn_particles,
};
use crate::plugins::texture_handling::{TextureAssets, TextureType};
use bevy::prelude::*;

pub struct WeaponEffectPlugin;

impl Plugin for WeaponEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_trail_effects,
                update_impact_effects,
                update_muzzle_flash,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct TrailEffect {
    pub spawn_timer: Timer,
    pub config: ParticleConfig,
}

#[derive(Component)]
pub struct ImpactEffect {
    pub radius: f32,
    pub lifetime: Timer,
}

#[derive(Component)]
pub struct MuzzleFlash {
    pub lifetime: Timer,
}

pub fn laser_trail_config(texture_assets: &TextureAssets) -> ParticleConfig {
    ParticleConfig {
        texture: Some(
            texture_assets
                .textures
                .get(&TextureType::Spark)
                .unwrap()
                .clone(),
        ),
        particle_lifetime: 0.3,
        velocity_min: Vec2::new(-20.0, -20.0),
        velocity_max: Vec2::new(20.0, 20.0),
        start_scale_min: 0.3,
        start_scale_max: 0.5,
        end_scale: 0.0,
        start_color: Color::srgb(0.0, 1.0, 0.0),
        end_color: Color::srgba(0.0, 0.5, 0.0, 0.0),
        rotation_speed_min: -5.0,
        rotation_speed_max: 5.0,
        gravity: 0.0,
        friction: 0.95,
        spawn_radius: 5.0,
    }
}

pub fn rocket_trail_config(texture_assets: &TextureAssets) -> ParticleConfig {
    ParticleConfig {
        texture: Some(
            texture_assets
                .textures
                .get(&TextureType::Smoke)
                .unwrap()
                .clone(),
        ),
        particle_lifetime: 0.8,
        velocity_min: Vec2::new(-30.0, -30.0),
        velocity_max: Vec2::new(30.0, 30.0),
        start_scale_min: 0.5,
        start_scale_max: 0.8,
        end_scale: 0.0,
        start_color: Color::srgb(1.0, 0.5, 0.8),
        end_color: Color::srgba(0.3, 0.3, 0.3, 0.0),
        rotation_speed_min: -2.0,
        rotation_speed_max: 2.0,
        gravity: -20.0,
        friction: 0.98,
        spawn_radius: 3.0,
    }
}

pub fn flame_config(texture_assets: &TextureAssets) -> ParticleConfig {
    ParticleConfig {
        texture: Some(
            texture_assets
                .textures
                .get(&TextureType::Flame)
                .unwrap()
                .clone(),
        ),
        particle_lifetime: 0.4,
        velocity_min: Vec2::new(-80.0, -80.0),
        velocity_max: Vec2::new(80.0, 80.0),
        start_scale_min: 0.3,
        start_scale_max: 0.6,
        end_scale: 0.1,
        start_color: Color::srgba(1.0, 0.6, 0.0, 0.9),
        end_color: Color::srgba(1.0, 0.0, 0.0, 0.0),
        rotation_speed_min: -8.0,
        rotation_speed_max: 8.0,
        gravity: -30.0,
        friction: 0.96,
        spawn_radius: 20.0,
    }
}

pub fn raygun_spark_config(texture_assets: &TextureAssets) -> ParticleConfig {
    ParticleConfig {
        texture: Some(
            texture_assets
                .textures
                .get(&TextureType::Electric)
                .unwrap()
                .clone(),
        ),
        particle_lifetime: 0.2,
        velocity_min: Vec2::new(-50.0, -50.0),
        velocity_max: Vec2::new(50.0, 50.0),
        start_scale_min: 0.8,
        start_scale_max: 1.5,
        end_scale: 0.0,
        start_color: Color::srgb(0.3, 1.0, 1.0),
        end_color: Color::srgba(0.5, 0.5, 1.0, 0.0),
        rotation_speed_min: -20.0,
        rotation_speed_max: 20.0,
        gravity: 0.0,
        friction: 0.85,
        spawn_radius: 0.0,
    }
}

pub fn sword_sparkle_config(texture_assets: &TextureAssets) -> ParticleConfig {
    ParticleConfig {
        texture: Some(
            texture_assets
                .textures
                .get(&TextureType::Sparkle)
                .unwrap()
                .clone(),
        ),
        particle_lifetime: 0.5,
        velocity_min: Vec2::new(-40.0, -40.0),
        velocity_max: Vec2::new(40.0, 40.0),
        start_scale_min: 0.8,
        start_scale_max: 0.9,
        end_scale: 0.0,
        start_color: Color::srgb(0.1, 1.0, 0.8),
        end_color: Color::srgba(1.0, 0.8, 0.0, 0.0),
        rotation_speed_min: -3.0,
        rotation_speed_max: 3.0,
        gravity: 20.0,
        friction: 0.97,
        spawn_radius: 15.0,
    }
}

pub fn attach_trail_effect(
    commands: &mut Commands,
    projectile_entity: Entity,
    weapon_type: WeaponType,
    texture_assets: &TextureAssets,
) {
    let config = match weapon_type {
        WeaponType::Laser => laser_trail_config(texture_assets),
        WeaponType::Rocket => rocket_trail_config(texture_assets),
        WeaponType::RayGun => raygun_spark_config(texture_assets),
        WeaponType::Sword => sword_sparkle_config(texture_assets),
        _ => return,
    };

    commands.entity(projectile_entity).insert(TrailEffect {
        spawn_timer: Timer::from_seconds(0.02, TimerMode::Repeating),
        config,
    });
}

pub fn spawn_explosion_effect(
    commands: &mut Commands,
    position: Vec3,
    radius: f32,
    texture_assets: &TextureAssets,
) {
    let mut config = rocket_trail_config(texture_assets);
    config.spawn_radius = radius * 0.5;

    commands.spawn((
        GameEntity,
        ParticleEmitter {
            enabled: true,
            spawn_timer: Timer::from_seconds(0.01, TimerMode::Repeating),
            particles_per_spawn: (radius * 2.0) as u32,
            config,
            offset: Vec3::ZERO,
            spawn_mode: SpawnMode::Circular {
                radius: radius - 15.0,
            },
            lifetime: Some(Timer::from_seconds(0.15, TimerMode::Once)),
        },
        Transform::from_translation(position),
    ));

    commands.spawn((
        GameEntity,
        ImpactEffect {
            radius,
            lifetime: Timer::from_seconds(0.3, TimerMode::Once),
        },
        Sprite {
            image: texture_assets
                .textures
                .get(&TextureType::ExplosionCore)
                .unwrap()
                .clone(),
            color: Color::srgba(1.0, 0.9, 0.5, 1.0),
            ..default()
        },
        Transform::from_translation(position + Vec3::Z * 20.0)
            .with_scale(Vec3::splat(radius * 0.1)),
    ));
}

pub fn spawn_impact_effects(
    commands: &mut Commands,
    position: Vec3,
    weapon_type: WeaponType,
    texture_assets: &TextureAssets,
) {
    let config = match weapon_type {
        WeaponType::Laser => {
            let mut c = laser_trail_config(texture_assets);
            c.velocity_min = Vec2::new(-80.0, -80.0);
            c.velocity_max = Vec2::new(80.0, 80.0);
            c
        }
        WeaponType::RayGun => raygun_spark_config(texture_assets),
        WeaponType::Sword => {
            let mut c = sword_sparkle_config(texture_assets);
            c.velocity_min = Vec2::new(-100.0, -100.0);
            c.velocity_max = Vec2::new(100.0, 100.0);
            c
        }
        _ => return,
    };

    for _ in 0..12 {
        spawn_particles(commands, texture_assets, position, &config)
    }
}

pub fn spawn_muzzle_flash(
    commands: &mut Commands,
    position: Vec3,
    direction: Vec3,
    texture_assets: &TextureAssets,
) {
    let angle = direction.y.atan2(direction.x);

    commands.spawn((
        GameEntity,
        MuzzleFlash {
            lifetime: Timer::from_seconds(0.01, TimerMode::Once),
        },
        Sprite {
            image: texture_assets
                .textures
                .get(&TextureType::MuzzleFlash)
                .unwrap()
                .clone(),
            color: Color::srgba(1.0, 0.9, 0.5, 1.0),
            ..default()
        },
        Transform::from_translation(position + Vec3::Z * 12.0)
            .with_rotation(Quat::from_rotation_z(angle))
            .with_scale(Vec3::new(0.5, 0.3, 1.0)),
    ));
}

pub fn update_trail_effects(
    mut commands: Commands,
    time: Res<Time>,
    texture_assets: Res<TextureAssets>,
    mut trails: Query<(&mut TrailEffect, &Transform)>,
) {
    for (mut trail, transform) in trails.iter_mut() {
        trail.spawn_timer.tick(time.delta());

        if trail.spawn_timer.just_finished() {
            spawn_particles(
                &mut commands,
                &texture_assets,
                transform.translation,
                &trail.config,
            );
        }
    }
}

pub fn update_impact_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut effects: Query<(Entity, &mut ImpactEffect, &mut Transform, &mut Sprite)>,
) {
    for (entity, mut effect, mut transform, mut sprite) in effects.iter_mut() {
        effect.lifetime.tick(time.delta());
        let progress = effect.lifetime.fraction();
        let scale = 1.0 + progress * effect.radius / 32.0;
        transform.scale = Vec3::splat(scale);
        sprite.color.set_alpha(1.0 - progress);

        if effect.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn update_muzzle_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut MuzzleFlash, &mut Sprite)>,
) {
    for (entity, mut flash, mut sprite) in flashes.iter_mut() {
        flash.lifetime.tick(time.delta());

        let progress = flash.lifetime.fraction();

        sprite.color.set_alpha(1.0 - progress);
        if flash.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
