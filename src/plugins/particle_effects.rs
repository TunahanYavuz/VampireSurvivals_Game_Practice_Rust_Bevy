use bevy::prelude::*;

use crate::plugins::common::GameEntity;
use crate::plugins::game_state::GameState;
use crate::plugins::texture_handling::{TextureAssets, TextureType};

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_particles, update_particle_emitters, cleanup_dead_particles,)
                .run_if(in_state(GameState::Playing)),
        );
    }
}


#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub lifetime: Timer,
    pub start_scale: f32,
    pub end_scale: f32,
    pub start_color: Color,
    pub end_color: Color,
    pub rotation_speed: f32,
    pub gravity: f32,
    pub friction: f32,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            lifetime: Timer::from_seconds(1.0, TimerMode::Once),
            start_scale: 1.0,
            end_scale: 0.0,
            start_color: Color::WHITE,
            end_color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            rotation_speed: 0.0,
            gravity: 0.0,
            friction: 0.98,
        }
    }
}


#[derive(Clone, Default)]
pub enum SpawnMode {
    #[default]
    Point,
    Circular {
        radius: f32,
    },
    Linear {
        start_point: Vec3,
        end_point: Vec3,
    },
    Box { size: Vec2 },
}

#[derive(Component)]
pub struct ParticleEmitter {
    pub enabled: bool,
    pub spawn_timer: Timer,
    pub particles_per_spawn: u32,
    pub config: ParticleConfig,
    pub offset: Vec3,
    pub spawn_mode: SpawnMode,
    pub lifetime: Option<Timer>,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            enabled: true,
            spawn_timer: Timer::from_seconds(0.05, TimerMode::Repeating),
            particles_per_spawn: 5,
            config: ParticleConfig::default(),
            offset: Vec3::ZERO,
            spawn_mode: SpawnMode::Point,
            lifetime: None,
        }
    }
}

#[derive(Clone)]
pub struct ParticleConfig {
    pub texture: Option<Handle<Image>>,
    pub particle_lifetime: f32,
    pub velocity_min: Vec2,
    pub velocity_max: Vec2,
    pub start_scale_min: f32,
    pub start_scale_max: f32,
    pub end_scale: f32,
    pub start_color: Color,
    pub end_color: Color,
    pub rotation_speed_min: f32,
    pub rotation_speed_max: f32,
    pub gravity: f32,
    pub friction: f32,
    pub spawn_radius: f32,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self{
            texture: None,
            particle_lifetime: 1.0,
            velocity_min: Vec2::new(-50.0, -50.0),
            velocity_max: Vec2::new(50.0, 50.0),
            start_scale_min: 1.0,
            start_scale_max: 1.0,
            end_scale: 0.0,
            start_color: Color::WHITE,
            end_color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            rotation_speed_min: 0.0,
            rotation_speed_max: 0.0,
            gravity: 0.0,
            friction: 1.0,
            spawn_radius: 0.0,
        }
    }
}

pub fn update_particles(
    time: Res<Time>,
    mut particles: Query<(&mut Particle, &mut Transform, &mut Sprite)>,
){
    for (mut particle, mut transform, mut sprite) in particles.iter_mut() {
        particle.lifetime.tick(time.delta());
        let progress = particle.lifetime.fraction();
        let p_acceleration = particle.acceleration * time.delta_secs();
        let p_friction = particle.friction;
        particle.velocity += p_acceleration;
        particle.velocity.y -= particle.gravity * time.delta_secs();
        particle.velocity *= p_friction;
        
        transform.translation += particle.velocity.extend(0.0) * time.delta_secs();
        
        transform.rotation *= Quat::from_rotation_z(particle.rotation_speed * time.delta_secs());
        
        let scale = particle.start_scale + (particle.end_scale - particle.start_scale) * progress;
        transform.scale = Vec3::splat(scale);

        let start = particle.start_color.to_srgba();
        let end = particle.end_color.to_srgba();
        sprite.color = Color::srgba(
            start.red + (end.red - start.red) * progress,
            start.green + (end.green - start.green) * progress,
            start.blue + (end.blue - start.blue) * progress,
            start.alpha + (end.alpha - start.alpha) * progress,
        );
    }
}

pub fn update_particle_emitters(
    mut commands: Commands,
    time: Res<Time>,
    texture_assets: Res<TextureAssets>,
    mut emitters: Query<(Entity, &mut ParticleEmitter, &Transform)>
) {
    for (entity, mut emitter, transform) in emitters.iter_mut() {
        if !emitter.enabled {
            continue;
        }

        emitter.spawn_timer.tick(time.delta());

        if emitter.spawn_timer.just_finished() {
            match &emitter.spawn_mode {
                SpawnMode::Point => {
                    // Normal spawn - tek noktada
                    let spawn_position = transform.translation + emitter.offset;
                    for _ in 0..emitter.particles_per_spawn {
                        spawn_particles(&mut commands, &texture_assets, spawn_position, &emitter.config);
                    }
                }
                SpawnMode::Circular { radius } => {
                    // Dairesel spawn - cember uzerinde
                    let angle_step = std::f32::consts::TAU / emitter.particles_per_spawn as f32;
                    for i in 0..emitter.particles_per_spawn {
                        let angle = angle_step * i as f32;
                        let offset = Vec3::new(
                            angle.cos() * radius,
                            angle.sin() * radius,
                            0.0
                        );
                        let spawn_position = transform.translation + offset + emitter.offset;
                        spawn_particles(&mut commands, &texture_assets, spawn_position, &emitter.config);
                    }
                }
                SpawnMode::Linear { start_point, end_point } => {
                    // Linear spawn - cizgi boyunca
                    for _ in 0..emitter.particles_per_spawn {
                        let t = rand_range(0.0, 1.0);
                        let spawn_position = transform.translation + start_point.lerp(*end_point, t);
                        spawn_particles(&mut commands, &texture_assets, spawn_position, &emitter.config);
                    }
                }
                SpawnMode::Box { size } => {
                    for _ in 0..emitter.particles_per_spawn {
                        let x = rand_range(-size.x/2.0, size.x/2.0 );
                        let y = rand_range(-size.y/2.0, size.y/2.0 );

                        let local_pos = Vec3::new(x, y, 0.0) + emitter.offset;
                        let spawn_position = transform.transform_point(local_pos);
                        spawn_particles(&mut commands, &texture_assets, spawn_position, &emitter.config);
                    }
                }
            }
        }

        if let Some(ref mut lifetime) = emitter.lifetime {
            lifetime.tick(time.delta());
            if lifetime.is_finished() {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn spawn_particles(
    commands: &mut Commands,
    texture_assets: &TextureAssets,
    position: Vec3,
    config: &ParticleConfig,
){
    let velocity = Vec2::new(
        rand_range(config.velocity_min.x, config.velocity_max.x),
        rand_range(config.velocity_min.y, config.velocity_max.y),
    );
    
    let start_scale = rand_range(config.start_scale_min, config.start_scale_max);
    let rotation_speed = rand_range(config.rotation_speed_min, config.rotation_speed_max);
    
    let offset = if config.spawn_radius > 0.0 {
        let angle = rand_range(0.0, std::f32::consts::TAU);
        let dist = rand_range(0.0, config.spawn_radius);
        Vec3::new(dist * angle.cos(), dist * angle.sin(), 0.0)
    } else {
        Vec3::ZERO
    };
    
    let texture = config.texture.clone().unwrap_or_else(|| texture_assets.textures.get(&TextureType::Particle).unwrap().clone());

    commands.spawn((
        GameEntity,
        Particle{
            velocity,
            acceleration: Vec2::ZERO,
            lifetime: Timer::from_seconds(config.particle_lifetime, TimerMode::Once),
            start_scale,
            end_scale: config.end_scale,
            start_color: config.start_color,
            end_color: config.end_color,
            rotation_speed,
            gravity: config.gravity,
            friction: config.friction,
        },
        Sprite {
            image: texture,
            color: config.start_color,
            ..default()
        },
        Transform::from_translation(position + offset + Vec3::Z * 15.0)
            .with_scale(Vec3::splat(start_scale)),
        ));
}

pub fn cleanup_dead_particles(
    mut commands: Commands,
    particles: Query<(Entity, &Particle)>,
){
    for (entity, particle) in particles.iter() {
        if particle.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn rand_range(min: f32, max: f32) -> f32 {
    min + rand::random::<f32>() * (max - min)
}