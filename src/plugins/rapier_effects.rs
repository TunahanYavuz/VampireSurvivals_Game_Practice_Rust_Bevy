use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use crate::plugins::common::GameEntity;
use crate::plugins::game_state::GameState;

/// Rapier tabanlı fizik efektleri sistemi
/// 
/// Bu plugin, Rapier physics engine kullanarak gerçekçi fizik efektleri sağlar:
/// - Fiziksel parçacıklar (çarpışma, yerçekimi, sürtünme)
/// - Patlama efektleri (radyal kuvvet dalgaları)
/// - Çarpışma algılama ve efekt tetikleme
/// - Fizik tabanlı partikül sistemleri
pub struct RapierEffectsPlugin;

impl Plugin for RapierEffectsPlugin {
    fn build(&self, app: &mut App) {
        app
            // Rapier physics plugin
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0));
        
        // Debug render sadece development modda
        #[cfg(debug_assertions)]
        app.add_plugins(RapierDebugRenderPlugin::default());
        
        app
            // Sistemler
            .add_systems(
                Update,
                (
                    update_physics_particles,
                    update_explosion_waves,
                    update_collision_effects,
                    cleanup_physics_particles,
                ).run_if(in_state(GameState::Playing))
            );
    }
}

/// Fizik tabanlı parçacık component
/// Rapier rigid body ile birlikte çalışır
#[derive(Component)]
pub struct PhysicsParticle {
    pub lifetime: Timer,
    pub start_color: Color,
    pub end_color: Color,
    pub start_scale: f32,
    pub end_scale: f32,
    pub friction_coefficient: f32,
}

impl Default for PhysicsParticle {
    fn default() -> Self {
        Self {
            lifetime: Timer::from_seconds(2.0, TimerMode::Once),
            start_color: Color::WHITE,
            end_color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            start_scale: 1.0,
            end_scale: 0.3,
            friction_coefficient: 0.7,
        }
    }
}

/// Patlama dalgası - radyal kuvvet uygular
#[derive(Component)]
pub struct ExplosionWave {
    pub radius: f32,
    pub force: f32,
    pub lifetime: Timer,
    pub affected_entities: Vec<Entity>,
}

/// Çarpışma efekt tetikleyici
/// Rigid body çarpışmalarında efekt oluşturur
#[derive(Component)]
pub struct CollisionEffectTrigger {
    pub particle_count: u32,
    pub color: Color,
    pub enabled: bool,
}

/// Fizik parçacık konfigürasyonu
#[derive(Clone)]
pub struct PhysicsParticleConfig {
    pub texture: Option<Handle<Image>>,
    pub lifetime: f32,
    pub velocity_min: Vec2,
    pub velocity_max: Vec2,
    pub angular_velocity_min: f32,
    pub angular_velocity_max: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub start_color: Color,
    pub end_color: Color,
    pub mass: f32,
    pub restitution: f32,  // Zıplama katsayısı (0.0-1.0)
    pub friction: f32,      // Sürtünme katsayısı (0.0-1.0)
    pub gravity_scale: f32, // Yerçekimi etkisi (1.0 = normal)
}

impl Default for PhysicsParticleConfig {
    fn default() -> Self {
        Self {
            texture: None,
            lifetime: 2.0,
            velocity_min: Vec2::new(-100.0, -100.0),
            velocity_max: Vec2::new(100.0, 100.0),
            angular_velocity_min: -5.0,
            angular_velocity_max: 5.0,
            start_scale: 1.0,
            end_scale: 0.3,
            start_color: Color::WHITE,
            end_color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            mass: 1.0,
            restitution: 0.5,
            friction: 0.7,
            gravity_scale: 1.0,
        }
    }
}

/// Fiziksel parçacık oluşturur (Rapier rigid body ile)
pub fn spawn_physics_particle(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    config: &PhysicsParticleConfig,
) {
    let velocity = Vec2::new(
        rand::random::<f32>() * (config.velocity_max.x - config.velocity_min.x) + config.velocity_min.x,
        rand::random::<f32>() * (config.velocity_max.y - config.velocity_min.y) + config.velocity_min.y,
    );
    
    let angular_velocity = rand::random::<f32>() * 
        (config.angular_velocity_max - config.angular_velocity_min) + config.angular_velocity_min;
    
    let texture = config.texture.clone()
        .unwrap_or_else(|| asset_server.load("effects/particle.png"));
    
    commands.spawn((
        GameEntity,
        PhysicsParticle {
            lifetime: Timer::from_seconds(config.lifetime, TimerMode::Once),
            start_color: config.start_color,
            end_color: config.end_color,
            start_scale: config.start_scale,
            end_scale: config.end_scale,
            friction_coefficient: config.friction,
        },
        // Rapier components
        RigidBody::Dynamic,
        Velocity {
            linvel: velocity,
            angvel: angular_velocity,
        },
        Collider::ball(10.0 * config.start_scale),
        Restitution::coefficient(config.restitution),
        Friction::coefficient(config.friction),
        GravityScale(config.gravity_scale),
        ColliderMassProperties::Mass(config.mass),
        // Rendering
        Sprite {
            image: texture,
            color: config.start_color,
            ..default()
        },
        Transform::from_translation(position)
            .with_scale(Vec3::splat(config.start_scale)),
    ));
}

/// Patlama efekti oluşturur - radyal kuvvet dalgası
pub fn spawn_explosion_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    radius: f32,
    force: f32,
    particle_count: u32,
    config: &PhysicsParticleConfig,
) {
    // Patlama dalgası
    commands.spawn((
        GameEntity,
        ExplosionWave {
            radius,
            force,
            lifetime: Timer::from_seconds(0.3, TimerMode::Once),
            affected_entities: Vec::new(),
        },
        Transform::from_translation(position),
    ));
    
    // Parçacıklar
    for i in 0..particle_count {
        let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
        let direction = Vec2::new(angle.cos(), angle.sin());
        
        let mut particle_config = config.clone();
        let speed = rand::random::<f32>() * 100.0 + 150.0;
        particle_config.velocity_min = direction * speed;
        particle_config.velocity_max = direction * (speed + 50.0);
        
        spawn_physics_particle(commands, asset_server, position, &particle_config);
    }
}

/// Çarpışma efekti oluşturur
pub fn spawn_collision_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    velocity: Vec2,
    config: &PhysicsParticleConfig,
    count: u32,
) {
    for _ in 0..count {
        let mut particle_config = config.clone();
        // Çarpışma yönüne göre parçacıkları dağıt
        let perpendicular = Vec2::new(-velocity.y, velocity.x).normalize_or_zero();
        let spread = 100.0;
        particle_config.velocity_min = velocity * -0.5 + perpendicular * -spread;
        particle_config.velocity_max = velocity * -0.3 + perpendicular * spread;
        
        spawn_physics_particle(commands, asset_server, position, &particle_config);
    }
}

/// Fiziksel parçacıkları günceller (renk, ölçek, ömür)
fn update_physics_particles(
    time: Res<Time>,
    mut particles: Query<(&mut PhysicsParticle, &mut Transform, &mut Sprite)>,
) {
    for (mut particle, mut transform, mut sprite) in particles.iter_mut() {
        particle.lifetime.tick(time.delta());
        let progress = particle.lifetime.fraction();
        
        // Ölçek interpolasyonu
        let scale = particle.start_scale + (particle.end_scale - particle.start_scale) * progress;
        transform.scale = Vec3::splat(scale);
        
        // Renk interpolasyonu
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

/// Patlama dalgalarını günceller - yakındaki rigid body'lere kuvvet uygular
fn update_explosion_waves(
    mut commands: Commands,
    time: Res<Time>,
    mut waves: Query<(Entity, &mut ExplosionWave, &Transform)>,
    mut bodies: Query<(&Transform, &mut Velocity), (With<RigidBody>, Without<ExplosionWave>)>,
) {
    for (entity, mut wave, wave_transform) in waves.iter_mut() {
        wave.lifetime.tick(time.delta());
        
        if !wave.lifetime.finished() {
            // Yakındaki tüm rigid body'lere radyal kuvvet uygula
            for (body_transform, mut velocity) in bodies.iter_mut() {
                let delta = body_transform.translation - wave_transform.translation;
                let distance = delta.truncate().length();
                
                if distance < wave.radius && distance > 0.0 {
                    // Mesafeye göre kuvvet azaltma
                    let force_scale = 1.0 - (distance / wave.radius);
                    let direction = delta.truncate().normalize();
                    let force = direction * wave.force * force_scale;
                    
                    velocity.linvel += force;
                }
            }
        }
        
        if wave.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Çarpışma algılama ve efekt tetikleme
fn update_collision_effects(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut collision_events: EventReader<CollisionEvent>,
    triggers: Query<&CollisionEffectTrigger>,
    transforms: Query<&Transform>,
    velocities: Query<&Velocity>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _flags) = event {
            // Her iki entity'yi kontrol et
            for entity in [e1, e2] {
                if let Ok(trigger) = triggers.get(*entity) {
                    if !trigger.enabled {
                        continue;
                    }
                    
                    // Çarpışma pozisyonu ve hızını al
                    if let (Ok(transform), Ok(velocity)) = 
                        (transforms.get(*entity), velocities.get(*entity)) {
                        
                        let config = PhysicsParticleConfig {
                            lifetime: 1.0,
                            start_color: trigger.color,
                            end_color: Color::srgba(
                                trigger.color.to_srgba().red, 
                                trigger.color.to_srgba().green, 
                                trigger.color.to_srgba().blue, 
                                0.0
                            ),
                            restitution: 0.3,
                            friction: 0.8,
                            gravity_scale: 0.5,
                            ..default()
                        };
                        
                        spawn_collision_effect(
                            &mut commands,
                            &asset_server,
                            transform.translation,
                            velocity.linvel,
                            &config,
                            trigger.particle_count,
                        );
                    }
                }
            }
        }
    }
}

/// Ömrü biten fiziksel parçacıkları temizler
fn cleanup_physics_particles(
    mut commands: Commands,
    particles: Query<(Entity, &PhysicsParticle)>,
) {
    for (entity, particle) in particles.iter() {
        if particle.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Örnek efekt konfigürasyonları
pub mod presets {
    use super::*;
    
    /// Ateş parçacıkları
    pub fn fire_particle_config(asset_server: &AssetServer) -> PhysicsParticleConfig {
        PhysicsParticleConfig {
            texture: Some(asset_server.load("effects/flame.png")),
            lifetime: 1.5,
            start_color: Color::srgb(1.0, 0.8, 0.0),
            end_color: Color::srgba(1.0, 0.0, 0.0, 0.0),
            restitution: 0.2,
            friction: 0.5,
            gravity_scale: -0.5, // Yukarı doğru
            ..default()
        }
    }
    
    /// Kıvılcım parçacıkları
    pub fn spark_particle_config(asset_server: &AssetServer) -> PhysicsParticleConfig {
        PhysicsParticleConfig {
            texture: Some(asset_server.load("effects/spark.png")),
            lifetime: 0.8,
            start_color: Color::srgb(1.0, 1.0, 0.0),
            end_color: Color::srgba(1.0, 0.5, 0.0, 0.0),
            restitution: 0.8,
            friction: 0.3,
            gravity_scale: 2.0,
            start_scale: 0.5,
            end_scale: 0.1,
            ..default()
        }
    }
    
    /// Duman parçacıkları
    pub fn smoke_particle_config(asset_server: &AssetServer) -> PhysicsParticleConfig {
        PhysicsParticleConfig {
            texture: Some(asset_server.load("effects/smoke.png")),
            lifetime: 3.0,
            start_color: Color::srgba(0.5, 0.5, 0.5, 0.8),
            end_color: Color::srgba(0.3, 0.3, 0.3, 0.0),
            restitution: 0.0,
            friction: 0.9,
            gravity_scale: -0.3,
            velocity_min: Vec2::new(-20.0, 20.0),
            velocity_max: Vec2::new(20.0, 50.0),
            ..default()
        }
    }
    
    /// Enkaz parçacıkları
    pub fn debris_particle_config(asset_server: &AssetServer) -> PhysicsParticleConfig {
        PhysicsParticleConfig {
            texture: Some(asset_server.load("effects/particle.png")),
            lifetime: 2.0,
            start_color: Color::srgb(0.6, 0.6, 0.6),
            end_color: Color::srgba(0.3, 0.3, 0.3, 0.0),
            restitution: 0.6,
            friction: 0.7,
            gravity_scale: 1.5,
            mass: 2.0,
            ..default()
        }
    }
}
