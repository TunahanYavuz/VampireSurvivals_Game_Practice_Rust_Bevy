use super::effects::{
    attach_trail_effect, raygun_spark_config, spawn_explosion_effect, spawn_impact_effects,
    spawn_muzzle_flash,
};
use super::stats::{Throwable, WeaponStats};
use super::upgrades::{WeaponLevel, WeaponType};
use crate::plugins::audio::{AudioType, PlayAudioEvent};
use crate::plugins::common::{GameEntity, aabb_intersects, contains_point};
use crate::plugins::enemy::Enemy;
use crate::plugins::game_state::GameState;
use crate::plugins::network::{
    NetIdCounter, NetOutbox, NetworkIdentity, NetworkRole, RemoteInput, S2C, TransformSnapshot,
    VisualType, encode,
};
use crate::plugins::particle_effects::{ParticleEmitter, SpawnMode};
use crate::plugins::player::Player;
use crate::plugins::texture_handling::{TextureAssets, TextureType};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                move_swords,
                fire_laser_weapons,
                fire_rocket_weapons,
                fire_raygun_weapons,
                move_projectiles,
                appy_flame_damage,
                update_raygun_rays,
                raygun_damage,
                throw_swords,
                despawn_lifetime_over,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// Temel silah component'i
#[derive(Component)]
pub struct Weapon {
    pub owner: Entity,
    pub damage: f32,
    pub fire_timer: Timer,
    pub speed: f32,
}

// Farklı silah tipleri - sadece özellikler
#[derive(Component, Clone, Copy, PartialEq)]
pub struct LaserWeapon {
    pub color: Color,
}

#[derive(Component)]
pub struct RayGunWeapon {
    pub color: Color,
    pub pierce_count: u32,
    pub targeted_enemies: Vec<Entity>,
    pub retarget_timer: Timer,
}

impl Default for RayGunWeapon {
    fn default() -> Self {
        Self {
            color: Color::srgb(0.0, 1.0, 1.0),
            pierce_count: 3,
            targeted_enemies: Vec::new(),
            retarget_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq)]
pub struct RocketWeapon {
    pub explosion_radius: f32,
    pub angle_index: u8,
}

/// Mermi tipi - sadece tip belirteci
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProjectileKind {
    Laser { color: Color },
    Rocket { explosion_radius: f32 },
}
#[derive(Component)]
pub struct Lifetime {
    timer: Timer,
}
impl Default for Lifetime {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        }
    }
}
impl Lifetime {
    fn new(seconds: f32, timer_mode: TimerMode) -> Self {
        Self {
            timer: Timer::from_seconds(seconds, timer_mode),
        }
    }
}

pub fn despawn_lifetime_over(
    mut commands: Commands,
    time: Res<Time>,
    mut q_lifetime: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in q_lifetime.iter_mut() {
        if lifetime.timer.tick(time.delta()).just_finished() {
            if lifetime.timer.mode() == TimerMode::Once {
                commands.entity(entity).try_despawn();
            }
        }
    }
}

// Mermi component'i
#[derive(Component)]
pub struct Projectile {
    pub direction: Vec3,
    pub speed: f32,
    pub damage: f32,
    pub kind: ProjectileKind,
}

#[derive(Component)]
pub struct FlameWeapon {
    pub radius: f32,
}

#[derive(Component)]
pub struct RayGunRay {
    pub target_entity: Entity,
    pub damage: f32,
    pub damage_timer: Timer,
    pub owner: Entity,
}

impl Default for RayGunRay {
    fn default() -> Self {
        Self {
            target_entity: Entity::PLACEHOLDER,
            damage: 10.0,
            damage_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            owner: Entity::PLACEHOLDER,
        }
    }
}

pub fn fire_raygun_weapons(
    mut commands: Commands,
    time: Res<Time>,
    mut weapons: Query<(&mut Weapon, &mut RayGunWeapon), With<RayGunWeapon>>,
    player: Query<&Transform, With<Player>>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    texture_assets: Res<TextureAssets>,
    mut net_id_counter: ResMut<NetIdCounter>,
    _role: Res<NetworkRole>,
    mut audio_events: MessageWriter<PlayAudioEvent>,
    _outbox: Option<ResMut<NetOutbox>>,
) {
    let mut played: bool = false;
    for (mut weapon, mut raygun) in weapons.iter_mut() {
        weapon.fire_timer.tick(time.delta());
        raygun.retarget_timer.tick(time.delta());

        let Ok(player_transform) = player.get(weapon.owner) else {
            continue;
        };

        if raygun.retarget_timer.just_finished() {
            raygun.targeted_enemies.retain(|&e| enemies.get(e).is_ok());

            if raygun.targeted_enemies.len() < raygun.pierce_count as usize {
                let mut sorted_enemies: Vec<(Entity, f32)> = enemies
                    .iter()
                    .filter(|(e, _)| !raygun.targeted_enemies.contains(e))
                    .map(|(e, t)| (e, player_transform.translation.distance(t.translation)))
                    .collect();
                sorted_enemies.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                for (enemy_entity, _) in sorted_enemies
                    .iter()
                    .take(raygun.pierce_count as usize - raygun.targeted_enemies.len())
                {
                    raygun.targeted_enemies.push(*enemy_entity);
                }
            }
        }

        if !weapon.fire_timer.just_finished() {
            continue;
        }

        for &enemy_entity in &raygun.targeted_enemies {
            let Ok((enemy_entity, enemy_transform)) = enemies.get(enemy_entity) else {
                continue;
            };

            let direction = enemy_transform.translation - player_transform.translation;
            let distance = direction.length();
            let angle = direction.y.atan2(direction.x);
            // RayGunRay spawn et - electric particle emitter ile birlikte
            commands.spawn((
                GameEntity,
                NetworkIdentity {
                    net_id: net_id_counter.next(),
                    visual_type: VisualType::RayGunRay,
                },
                RayGunRay {
                    target_entity: enemy_entity,
                    damage: weapon.damage,
                    owner: weapon.owner,
                    ..default()
                },
                Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(raygun.color))),
                Transform {
                    translation: (player_transform.translation + direction / 2.0).with_z(1.0),
                    rotation: Quat::from_rotation_z(angle),
                    scale: Vec3::new(distance, 5.0, 1.0),
                },
                GlobalTransform::default(),
                // Electric particle emitter - çizgi boyunca
                ParticleEmitter {
                    enabled: true,
                    spawn_timer: Timer::from_seconds(0.04, TimerMode::Repeating),
                    particles_per_spawn: distance.abs() as u32 / 10_u32,
                    config: raygun_spark_config(&texture_assets),
                    offset: Vec3::ZERO,
                    spawn_mode: SpawnMode::Box {
                        size: Vec2::new(1.0, 0.2),
                    },
                    lifetime: None,
                },
            ));
            if !played {
                audio_events.write(PlayAudioEvent {
                    audio_type: AudioType::RaygunRayFire,
                });
                played = true;
            }
        }
    }
}

pub fn update_raygun_rays(
    mut raygun_q: Query<
        (&RayGunRay, &mut Transform),
        (With<RayGunRay>, Without<Player>, Without<Enemy>),
    >,
    enemies: Query<&Transform, (With<Enemy>, Without<Player>)>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    for (ray, mut ray_transform) in raygun_q.iter_mut() {
        let Ok(player_transform) = player.get(ray.owner) else {
            continue;
        };
        ray_transform.translation = player_transform.translation.with_z(10.0);

        if let Ok(enemy_transform) = enemies.get(ray.target_entity) {
            let direction = enemy_transform.translation - player_transform.translation;
            let distance = direction.length();
            let angle = direction.y.atan2(direction.x);
            ray_transform.translation =
                (player_transform.translation + direction / 2.0).with_z(1.0);
            ray_transform.rotation = Quat::from_rotation_z(angle);
            ray_transform.scale = Vec3::new(distance, 5.0, 1.0);
        }
    }
}

pub fn raygun_damage(
    mut raygun_q: Query<(&mut RayGunRay, &Transform, Entity), With<RayGunRay>>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform), (With<Enemy>, Without<Player>)>,
    time: Res<Time>,
    mut commands: Commands,
    mut raygun_weapons: Query<&mut RayGunWeapon, With<RayGunWeapon>>,
    texture_assets: Res<TextureAssets>,
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
) {
    // Ölen düşmanları topla
    let mut dead_enemies = Vec::new();

    // İlk geçiş: damage uygula ve ölen düşmanları topla
    for (mut raygun, transform, raygun_entity) in raygun_q.iter_mut() {
        raygun.damage_timer.tick(time.delta());

        if !raygun.damage_timer.just_finished() {
            continue;
        }

        // Düşman hâlâ var mı kontrol et
        let Ok((enemy_entity, mut enemy, enemy_transform)) = enemies.get_mut(raygun.target_entity)
        else {
            // Düşman yok, ray'i sil
            commands.entity(raygun_entity).despawn();
            continue;
        };

        enemy.health = enemy.health.saturating_sub(raygun.damage as i32);

        // Düşman üzerinde spark efekti
        spawn_impact_effects(
            &mut commands,
            enemy_transform.translation,
            WeaponType::RayGun,
            &texture_assets,
        );
        if *role == NetworkRole::Host {
            if let Some(ref outbox) = outbox {
                // Impact efekti için Trail veya yeni bir Impact visual_type yollayabilirsin
                let event = S2C::WeaponFxSpawned {
                    visual_type: VisualType::Trail, // Eger Trail impact anlamına geliyorsa
                    transform: TransformSnapshot::from_transform(&Transform::from_translation(
                        transform.translation,
                    )),
                    owner_net_id: None,
                };
                if let Ok(bytes) = encode(&event) {
                    let _ = outbox.0.send(bytes);
                }
            }
        }

        if enemy.health <= 0 {
            dead_enemies.push(enemy_entity);
        }
    }

    // İkinci geçiş: ölen düşmanların raylerini temizle
    if !dead_enemies.is_empty() {
        for (raygun, _transform, raygun_entity) in raygun_q.iter() {
            if dead_enemies.contains(&raygun.target_entity) {
                commands.entity(raygun_entity).despawn();
            }
        }

        // Silahlardan ölen düşmanları temizle
        for mut raygun_weapon in raygun_weapons.iter_mut() {
            raygun_weapon
                .targeted_enemies
                .retain(|&e| !dead_enemies.contains(&e));
        }
    }
}

// Lazer silahlarını ateşle
pub fn fire_laser_weapons(
    mut commands: Commands,
    time: Res<Time>,
    mut weapons: Query<(&mut Weapon, &LaserWeapon, &WeaponLevel, &WeaponStats), With<LaserWeapon>>,
    players: Query<&Transform, With<Player>>,
    enemies: Query<&Transform, With<Enemy>>,
    texture_assets: Res<TextureAssets>,
    mut net_id_counter: ResMut<NetIdCounter>,
    role: Res<NetworkRole>,
    mut audio_events: MessageWriter<PlayAudioEvent>,
    outbox: Option<Res<NetOutbox>>,
) {
    for (mut weapon, laser, level, stats) in weapons.iter_mut() {
        weapon.fire_timer.tick(time.delta());

        if !weapon.fire_timer.just_finished() {
            continue;
        }

        // Owner player'ı bul
        let Ok(player_transform) = players.get(weapon.owner) else {
            continue;
        };

        // En yakın düşmanı bul
        let Some(target_pos) = find_nearest_enemy(player_transform.translation, &enemies) else {
            continue;
        };

        let direction = (target_pos - player_transform.translation).normalize();
        let angle = direction.y.atan2(direction.x);
        spawn_muzzle_flash(
            &mut commands,
            player_transform.translation,
            direction,
            &texture_assets,
        );

        if *role == NetworkRole::Host {
            if let Some(ref outbox) = outbox {
                let event = S2C::WeaponFxSpawned {
                    visual_type: VisualType::MuzzleFlash { direction },
                    transform: TransformSnapshot::from_transform(&Transform::from_translation(
                        player_transform.translation,
                    )),
                    owner_net_id: None,
                };
                if let Ok(bytes) = encode(&event) {
                    let _ = outbox.0.send(bytes);
                }
            }
        }

        let projectile_count = stats.projectile_count(level.level).max(1);
        let spread = 0.16_f32;
        for i in 0..projectile_count {
            let centered = i as f32 - (projectile_count as f32 - 1.0) * 0.5;
            let shot_angle = angle + centered * spread;
            let shot_dir = Vec3::new(shot_angle.cos(), shot_angle.sin(), 0.0).normalize();
            let projectile_entity = commands
                .spawn((
                    GameEntity,
                    NetworkIdentity {
                        net_id: net_id_counter.next(),
                        visual_type: VisualType::LaserProjectile,
                    },
                    Projectile {
                        direction: shot_dir,
                        speed: weapon.speed,
                        damage: weapon.damage,
                        kind: ProjectileKind::Laser { color: laser.color },
                    },
                    Lifetime::new(3.0, TimerMode::Once),
                    Sprite::from_image(
                        texture_assets
                            .textures
                            .get(&TextureType::Laser)
                            .unwrap()
                            .clone(),
                    ),
                    Transform::from_translation(
                        player_transform.translation + Vec3::new(0.0, 0.0, 10.0),
                    )
                    .with_rotation(Quat::from_rotation_z(shot_angle)),
                    GlobalTransform::default(),
                ))
                .id();
            attach_trail_effect(
                &mut commands,
                projectile_entity,
                WeaponType::Laser,
                &texture_assets,
            );
        }
        audio_events.write(PlayAudioEvent {
            audio_type: AudioType::LaserProjectileFire,
        });
    }
}

// Roket silahlarını ateşle
pub fn fire_rocket_weapons(
    mut commands: Commands,
    time: Res<Time>,
    mut weapons: Query<
        (&mut Weapon, &mut RocketWeapon, &WeaponLevel, &WeaponStats),
        With<RocketWeapon>,
    >,
    players: Query<&Transform, With<Player>>,
    texture_assets: Res<TextureAssets>,
    mut net_id_counter: ResMut<NetIdCounter>,
    role: Res<NetworkRole>,
    mut audio_events: MessageWriter<PlayAudioEvent>,
    outbox: Option<Res<NetOutbox>>,
) {
    for (mut weapon, mut rocket, level, stats) in weapons.iter_mut() {
        weapon.fire_timer.tick(time.delta());

        if !weapon.fire_timer.just_finished() {
            continue;
        }

        let Ok(player_transform) = players.get(weapon.owner) else {
            continue;
        };

        let angles_deg = [0.0_f32, 60.0, 120.0, 180.0, 240.0, 300.0];
        let angle_deg = angles_deg[(rocket.angle_index % angles_deg.len() as u8) as usize];
        let angle_rad = angle_deg.to_radians();
        let direction = Vec3::new(angle_rad.cos(), angle_rad.sin(), 0.0);
        let angle = direction.y.atan2(direction.x);

        spawn_muzzle_flash(
            &mut commands,
            player_transform.translation,
            direction,
            &texture_assets,
        );
        if *role == NetworkRole::Host {
            if let Some(ref outbox) = outbox {
                let event = S2C::WeaponFxSpawned {
                    visual_type: VisualType::MuzzleFlash {
                        direction: direction,
                    },
                    transform: TransformSnapshot::from_transform(&Transform::from_translation(
                        player_transform.translation,
                    )),
                    owner_net_id: None,
                };
                if let Ok(bytes) = encode(&event) {
                    let _ = outbox.0.send(bytes);
                }
            }
        }

        let projectile_count = stats.projectile_count(level.level).max(1);
        let spread = 0.22_f32;
        for i in 0..projectile_count {
            let centered = i as f32 - (projectile_count as f32 - 1.0) * 0.5;
            let shot_angle = angle + centered * spread;
            let shot_dir = Vec3::new(shot_angle.cos(), shot_angle.sin(), 0.0).normalize();
            let projectile_entity = commands
                .spawn((
                    GameEntity,
                    NetworkIdentity {
                        net_id: net_id_counter.next(),
                        visual_type: VisualType::RocketProjectile,
                    },
                    Projectile {
                        direction: shot_dir,
                        speed: weapon.speed,
                        damage: weapon.damage,
                        kind: ProjectileKind::Rocket {
                            explosion_radius: rocket.explosion_radius,
                        },
                    },
                    Lifetime::new(5.0, TimerMode::Once),
                    Sprite::from_image(
                        texture_assets
                            .textures
                            .get(&TextureType::Rocket)
                            .unwrap()
                            .clone(),
                    ),
                    Transform::from_translation(
                        player_transform.translation + Vec3::new(0.0, 0.0, 10.0),
                    )
                    .with_rotation(Quat::from_rotation_z(shot_angle)),
                    GlobalTransform::default(),
                ))
                .id();
            attach_trail_effect(
                &mut commands,
                projectile_entity,
                WeaponType::Rocket,
                &texture_assets,
            );
        }
        rocket.angle_index += 1;
        audio_events.write(PlayAudioEvent {
            audio_type: AudioType::RocketProjectileFire,
        });
    }
}

pub fn appy_flame_damage(
    time: Res<Time>,
    mut flame_weapon: Query<
        (
            &GlobalTransform,
            &WeaponStats,
            &mut Weapon,
            &FlameWeapon,
            Entity,
            &mut ParticleEmitter,
        ),
        With<FlameWeapon>,
    >,
    mut enemies: Query<(&Transform, &mut Enemy), Without<FlameWeapon>>,
) {
    for (global_transform, _weapon_stats, mut weapon, addicted_comp, _w_entity, _emitter) in
        flame_weapon.iter_mut()
    {
        weapon.fire_timer.tick(time.delta());
        if !weapon.fire_timer.just_finished() {
            continue;
        }

        let weapon_radius = addicted_comp.radius;
        for (enemy_transform, mut enemy) in enemies.iter_mut() {
            let dist = enemy_transform
                .translation
                .distance(global_transform.translation());
            if dist <= weapon_radius {
                enemy.health = enemy.health.saturating_sub(weapon.damage as i32);
            }
        }
    }
}

// Mermileri hareket ettir ve çarpışma kontrolü yap
pub fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile), With<Projectile>>,
    mut enemies: Query<(&mut Transform, &mut Enemy, &mut Aabb), Without<Projectile>>,
    texture_assets: Res<TextureAssets>,
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
) {
    for (proj_entity, mut proj_transform, projectile) in projectiles.iter_mut() {
        // Hareketı uygula
        proj_transform.translation += projectile.direction * projectile.speed * time.delta_secs();

        // Düşman çarpışma kontrolü

        match &projectile.kind {
            ProjectileKind::Laser { .. } => {
                for (mut enemy_transform, mut enemy, enemy_aabb) in enemies.iter_mut() {
                    if contains_point(&enemy_aabb, proj_transform.translation) {
                        // Knockback
                        enemy_transform.translation += projectile.direction * 10.;
                        // Hasar
                        enemy.health = enemy.health.saturating_sub(projectile.damage as i32);

                        spawn_impact_effects(
                            &mut commands,
                            proj_transform.translation,
                            WeaponType::Laser,
                            &texture_assets,
                        );
                        if *role == NetworkRole::Host {
                            if let Some(ref outbox) = outbox {
                                // Impact efekti için Trail veya yeni bir Impact visual_type yollayabilirsin
                                let event = S2C::WeaponFxSpawned {
                                    visual_type: VisualType::Trail, // Eger Trail impact anlamına geliyorsa
                                    transform: TransformSnapshot::from_transform(
                                        &Transform::from_translation(proj_transform.translation),
                                    ),
                                    owner_net_id: None,
                                };
                                if let Ok(bytes) = encode(&event) {
                                    let _ = outbox.0.send(bytes);
                                }
                            }
                        }

                        // Mermi yok et
                        commands.entity(proj_entity).try_despawn();
                        // Düşman öldüyse

                        break;
                    }
                }
            }
            ProjectileKind::Rocket { explosion_radius } => {
                // Önce roketin herhangi bir düşmana çarpıp çarpmadığını kontrol et
                let mut explosion_pos: Option<Vec3> = None;

                for (_enemy_transform, _enemy, enemy_aabb) in enemies.iter() {
                    if contains_point(enemy_aabb, proj_transform.translation) {
                        // Roket bir düşmana çarptı, patlama konumunu kaydet
                        explosion_pos = Some(proj_transform.translation);
                        break;
                    }
                }

                // Eğer patlama olduysa, patlama yarıçapındaki TÜM düşmanlara hasar ver
                if let Some(explosion_center) = explosion_pos {
                    // Patlama görselini oluştur
                    spawn_explosion_effect(
                        &mut commands,
                        explosion_center,
                        *explosion_radius,
                        &texture_assets,
                    );
                    if *role == NetworkRole::Host {
                        if let Some(ref outbox) = outbox {
                            let event = S2C::WeaponFxSpawned {
                                visual_type: VisualType::Explosion {
                                    radius: *explosion_radius,
                                },
                                transform: TransformSnapshot::from_transform(
                                    &Transform::from_translation(explosion_center),
                                ),
                                owner_net_id: None,
                            };
                            if let Ok(bytes) = encode(&event) {
                                let _ = outbox.0.send(bytes);
                            }
                        }
                    }

                    // commands.spawn((
                    //     GameEntity,
                    //     Mesh2d(meshes.add(Circle::new(*explosion_radius))),
                    //     MeshMaterial2d(
                    //         materials.add(ColorMaterial::from(Color::srgba(1.0, 0.1, 0.0, 0.3))),
                    //     ),
                    //     Transform::from_translation(explosion_center),
                    //     Explosion {
                    //         lifetime: Timer::from_seconds(0.2, TimerMode::Once),
                    //     },
                    // ));

                    // Tüm düşmanları tekrar tara ve patlama yarıçapındakilere hasar ver
                    for (mut enemy_transform, mut enemy, _enemy_aabb) in enemies.iter_mut() {
                        let dist = enemy_transform.translation.distance(explosion_center);
                        if dist <= *explosion_radius {
                            // Knockback - patlamadan uzağa it
                            let knockback_dir = (enemy_transform.translation - explosion_center)
                                .normalize_or_zero();
                            enemy_transform.translation += knockback_dir * 20.;

                            // Hasar
                            enemy.health = enemy.health.saturating_sub(projectile.damage as i32);
                        }
                    }

                    // Roketi sil
                    commands.entity(proj_entity).try_despawn();
                }
            }
        }
    }
}

#[derive(Component)]
pub struct SwordProjectile {
    pub direction: Vec3,
    pub speed: f32,
    pub damage: f32,
    pub lifetime: Timer,
    pub angle: f32,
    pub hit_enemies: Vec<Entity>,
}

pub fn throw_swords(
    mut commands: Commands,
    time: Res<Time>,
    players: Query<(&Player, &Transform), (With<Player>, Without<Throwable>)>,
    mut sword: Query<(&mut Throwable, &mut Weapon), (With<Throwable>, Without<Player>)>,
    window: Single<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    texture_assets: Res<TextureAssets>,
    mut net_id_counter: ResMut<NetIdCounter>,
    r_input: Res<RemoteInput>,
    _role: Res<NetworkRole>,
    mut audio_events: MessageWriter<PlayAudioEvent>,
    _outbox: Option<ResMut<NetOutbox>>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };

    // Resolve cursor world position once per frame.
    let cursor_world_pos = window
        .cursor_position()
        .and_then(|pos| camera.viewport_to_world_2d(camera_transform, pos).ok());

    for (mut sword_comp, mut weapon) in sword.iter_mut() {
        weapon.fire_timer.tick(time.delta());
        if !weapon.fire_timer.just_finished() {
            continue;
        }

        // Look up the player that owns this sword weapon.
        let Ok((p, player_transform)) = players.get(weapon.owner) else {
            continue;
        };

        let direction = if let Some(r_mwp) = r_input.0.mouse_world_pos
            && p.player_index == 1
        {
            (Vec2::new(r_mwp[0], r_mwp[1]) - player_transform.translation.xy())
                .normalize()
                .extend(0.0)
        } else {
            if let Some(world_pos) = cursor_world_pos
                && p.player_index == 0
            {
                (world_pos - player_transform.translation.xy())
                    .normalize()
                    .extend(0.0)
            } else {
                sword_comp.last_direction
            }
        };

        sword_comp.last_direction = direction;

        let projectile_entity = commands
            .spawn((
                GameEntity,
                NetworkIdentity {
                    net_id: net_id_counter.next(),
                    visual_type: VisualType::SwordWeapon,
                },
                SwordProjectile {
                    angle: 0.0,
                    direction,
                    speed: weapon.speed,
                    damage: weapon.damage,
                    hit_enemies: Vec::new(),
                    lifetime: Timer::from_seconds(2.0, TimerMode::Once),
                },
                Aabb {
                    center: player_transform.translation.to_vec3a(),
                    half_extents: Vec3A::splat(60.0),
                },
                NoFrustumCulling,
                Sprite {
                    image: texture_assets
                        .textures
                        .get(&TextureType::Sword)
                        .unwrap()
                        .clone(),
                    ..default()
                },
                Transform::from_translation(player_transform.translation),
            ))
            .id();
        attach_trail_effect(
            &mut commands,
            projectile_entity,
            WeaponType::Sword,
            &texture_assets,
        );
        audio_events.write(PlayAudioEvent {
            audio_type: AudioType::SwordProjectileFire,
        });
    }
}

pub fn move_swords(
    mut commands: Commands,
    time: Res<Time>,
    mut swords: Query<
        (Entity, &mut Transform, &mut SwordProjectile, &mut Aabb),
        (With<SwordProjectile>, Without<Enemy>),
    >,
    mut enemies: Query<
        (Entity, &mut Enemy, &mut Aabb, &mut Transform),
        (With<Enemy>, Without<SwordProjectile>),
    >,
    texture_assets: Res<TextureAssets>,
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
    mut audio_events: MessageWriter<PlayAudioEvent>,
) {
    for (sword_entity, mut sword_transform, mut sword, mut sword_aabb) in swords.iter_mut() {
        // Ömür kontrolü
        sword.lifetime.tick(time.delta());
        if sword.lifetime.just_finished() {
            commands.entity(sword_entity).despawn();
        }

        // Hareket
        sword_transform.translation += sword.direction * sword.speed * time.delta_secs();
        sword_aabb.center = sword_transform.translation.to_vec3a();
        // Döndürme efekti
        sword.angle += 13.0 * time.delta_secs();
        sword_transform.rotation = Quat::from_rotation_z(sword.angle);

        for (enemy_entity, mut enemy, mut aabb, mut transform) in enemies.iter_mut() {
            if aabb_intersects(&aabb, &sword_aabb) && !sword.hit_enemies.contains(&enemy_entity) {
                // Hasar uygula
                enemy.health = enemy.health.saturating_sub(sword.damage as i32);

                transform.translation += sword.direction * 10.0;
                aabb.center = transform.translation.to_vec3a();
                sword.hit_enemies.push(enemy_entity);
                spawn_impact_effects(
                    &mut commands,
                    sword_transform.translation,
                    WeaponType::Sword,
                    &texture_assets,
                );
                audio_events.write(PlayAudioEvent {
                    audio_type: AudioType::SwordProjectileImpact,
                });
                if *role == NetworkRole::Host {
                    if let Some(ref outbox) = outbox {
                        // Impact efekti için Trail veya yeni bir Impact visual_type yollayabilirsin
                        let event = S2C::WeaponFxSpawned {
                            visual_type: VisualType::Trail, // Eger Trail impact anlamına geliyorsa
                            transform: TransformSnapshot::from_transform(
                                &Transform::from_translation(sword_transform.translation),
                            ),
                            owner_net_id: None,
                        };
                        if let Ok(bytes) = encode(&event) {
                            let _ = outbox.0.send(bytes);
                        }
                    }
                }
                break;
            }
        }
    }
}

// Yardımcı fonksiyon - en yakın düşmanı bul
fn find_nearest_enemy(position: Vec3, enemies: &Query<&Transform, With<Enemy>>) -> Option<Vec3> {
    enemies
        .iter()
        .map(|t| (t.translation, position.distance(t.translation)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(pos, _)| pos)
}
