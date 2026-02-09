use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use crate::plugins::common::GameEntity;
use crate::plugins::game_state::GameState;
use crate::plugins::rapier_effects::{
    spawn_explosion_effect, spawn_physics_particle, 
    CollisionEffectTrigger, PhysicsParticleConfig, presets
};

/// Demo plugin - Rapier efekt sistemini test etmek için
/// 
/// Klavye kısayolları:
/// - E: Patlama efekti
/// - F: Ateş parçacıkları
/// - S: Kıvılcım efekti
/// - D: Duman efekti
/// - R: Enkaz parçacıkları
/// - Space: Çarpışma efekti olan fiziksel top oluştur
pub struct RapierEffectsDemoPlugin;

impl Plugin for RapierEffectsDemoPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Update,
                (
                    demo_keyboard_input,
                    demo_auto_explosion_spawner,
                ).run_if(in_state(GameState::Playing))
            );
    }
}

/// Klavye girişlerini dinler ve demo efektler oluşturur
fn demo_keyboard_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    // Kamera pozisyonunu al (orta nokta)
    let camera_transform = camera_query.get_single();
    let spawn_position = if let Ok(transform) = camera_transform {
        transform.translation
    } else {
        Vec3::ZERO
    };
    
    // E tuşu - Patlama efekti
    if keyboard.just_pressed(KeyCode::KeyE) {
        info!("🎆 Patlama efekti oluşturuluyor...");
        let config = presets::fire_particle_config(&asset_server);
        spawn_explosion_effect(
            &mut commands,
            &asset_server,
            spawn_position,
            150.0,    // radius
            3000.0,   // force
            24,       // particle count
            &config,
        );
    }
    
    // F tuşu - Ateş parçacıkları
    if keyboard.just_pressed(KeyCode::KeyF) {
        info!("🔥 Ateş parçacıkları oluşturuluyor...");
        let config = presets::fire_particle_config(&asset_server);
        for _ in 0..10 {
            spawn_physics_particle(&mut commands, &asset_server, spawn_position, &config);
        }
    }
    
    // S tuşu - Kıvılcım efekti
    if keyboard.just_pressed(KeyCode::KeyS) {
        info!("⚡ Kıvılcım efekti oluşturuluyor...");
        let config = presets::spark_particle_config(&asset_server);
        for _ in 0..15 {
            spawn_physics_particle(&mut commands, &asset_server, spawn_position, &config);
        }
    }
    
    // D tuşu - Duman efekti
    if keyboard.just_pressed(KeyCode::KeyD) {
        info!("💨 Duman efekti oluşturuluyor...");
        let config = presets::smoke_particle_config(&asset_server);
        for _ in 0..8 {
            spawn_physics_particle(&mut commands, &asset_server, spawn_position, &config);
        }
    }
    
    // R tuşu - Enkaz parçacıkları
    if keyboard.just_pressed(KeyCode::KeyR) {
        info!("🪨 Enkaz parçacıkları oluşturuluyor...");
        let config = presets::debris_particle_config(&asset_server);
        for _ in 0..12 {
            spawn_physics_particle(&mut commands, &asset_server, spawn_position, &config);
        }
    }
    
    // Space tuşu - Çarpışma efekti olan top
    if keyboard.just_pressed(KeyCode::Space) {
        info!("⚽ Fiziksel top (çarpışma efektli) oluşturuluyor...");
        spawn_physics_ball_with_collision_effect(&mut commands, &asset_server, spawn_position);
    }
}

/// Çarpışma efekti olan fiziksel top oluşturur
fn spawn_physics_ball_with_collision_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
) {
    let velocity = Vec2::new(
        (rand::random::<f32>() - 0.5) * 400.0,
        (rand::random::<f32>() - 0.5) * 400.0,
    );
    
    commands.spawn((
        GameEntity,
        // Rapier bileşenleri
        RigidBody::Dynamic,
        Velocity {
            linvel: velocity,
            angvel: (rand::random::<f32>() - 0.5) * 10.0,
        },
        Collider::ball(15.0),
        Restitution::coefficient(0.8),
        Friction::coefficient(0.5),
        GravityScale(1.0),
        ColliderMassProperties::Mass(1.0),
        // Çarpışma efekt tetikleyici
        CollisionEffectTrigger {
            particle_count: 8,
            color: Color::srgb(1.0, 1.0, 0.0),
            enabled: true,
        },
        // Görsel
        Sprite {
            image: asset_server.load("effects/particle.png"),
            color: Color::srgb(0.5, 0.8, 1.0),
            ..default()
        },
        Transform::from_translation(position)
            .with_scale(Vec3::splat(30.0)),
    ));
}

/// Otomatik patlama efekti oluşturucu (demo amaçlı)
/// Her 10 saniyede bir rastgele pozisyonda patlama oluşturur
fn demo_auto_explosion_spawner(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    // Timer'ı ilk çalıştırmada oluştur
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(10.0, TimerMode::Repeating));
    }
    
    if let Some(ref mut t) = timer.as_mut() {
        t.tick(time.delta());
        
        if t.just_finished() {
            // Kamera pozisyonuna göre rastgele bir yer seç
            let camera_transform = camera_query.get_single();
            let base_position = if let Ok(transform) = camera_transform {
                transform.translation
            } else {
                Vec3::ZERO
            };
            
            // Rastgele offset ekle
            let offset = Vec3::new(
                (rand::random::<f32>() - 0.5) * 400.0,
                (rand::random::<f32>() - 0.5) * 400.0,
                0.0,
            );
            let spawn_position = base_position + offset;
            
            info!("💥 Otomatik patlama efekti! Pozisyon: {:?}", spawn_position);
            
            // Rastgele preset seç
            let configs = [
                presets::fire_particle_config(&asset_server),
                presets::spark_particle_config(&asset_server),
                presets::smoke_particle_config(&asset_server),
                presets::debris_particle_config(&asset_server),
            ];
            let config = configs[rand::random::<usize>() % configs.len()].clone();
            
            spawn_explosion_effect(
                &mut commands,
                &asset_server,
                spawn_position,
                100.0 + rand::random::<f32>() * 100.0,  // Rastgele yarıçap
                2000.0 + rand::random::<f32>() * 2000.0, // Rastgele kuvvet
                16 + (rand::random::<u32>() % 16),      // Rastgele parçacık sayısı
                &config,
            );
        }
    }
}

/// Zemin/platform oluşturur (parçacıkların zıplaması için)
pub fn spawn_demo_ground(
    commands: &mut Commands,
    asset_server: &AssetServer,
) {
    // Ana zemin
    commands.spawn((
        GameEntity,
        RigidBody::Fixed,
        Collider::cuboid(1000.0, 50.0),
        Friction::coefficient(0.7),
        Restitution::coefficient(0.3),
        Sprite {
            image: asset_server.load("ground/tile.png"),
            color: Color::srgb(0.3, 0.3, 0.3),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -300.0, 0.0))
            .with_scale(Vec3::new(2000.0, 100.0, 1.0)),
    ));
    
    // Sol duvar
    commands.spawn((
        GameEntity,
        RigidBody::Fixed,
        Collider::cuboid(50.0, 1000.0),
        Friction::coefficient(0.7),
        Restitution::coefficient(0.3),
        Transform::from_translation(Vec3::new(-500.0, 0.0, 0.0)),
    ));
    
    // Sağ duvar
    commands.spawn((
        GameEntity,
        RigidBody::Fixed,
        Collider::cuboid(50.0, 1000.0),
        Friction::coefficient(0.7),
        Restitution::coefficient(0.3),
        Transform::from_translation(Vec3::new(500.0, 0.0, 0.0)),
    ));
}

/// Demo UI - tuş açıklamaları
pub fn spawn_demo_ui(mut commands: Commands) {
    let text_style = TextFont {
        font_size: 20.0,
        ..default()
    };
    
    commands.spawn((
        Text::new(
            "Rapier Efekt Demo\n\
            E - Patlama Efekti\n\
            F - Ateş Parçacıkları\n\
            S - Kıvılcım Efekti\n\
            D - Duman Efekti\n\
            R - Enkaz Parçacıkları\n\
            Space - Fiziksel Top (Çarpışma Efektli)\n\
            \n\
            Otomatik patlamalar: Her 10 saniyede"
        ),
        text_style,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
    ));
}
