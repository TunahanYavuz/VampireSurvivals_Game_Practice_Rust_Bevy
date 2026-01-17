
use crate::plugins::common::{GameEntity, aabb_intersects, contains_point};
use crate::plugins::enemy::Enemy;
use crate::plugins::game_state::GameState;
use crate::plugins::player::Player;
use crate::plugins::texture_handling::TextureAssets;
use crate::plugins::weapon_stats::{SwordWeapon, WeaponStats};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{ NoFrustumCulling};
use bevy::mesh::{Indices, PrimitiveTopology};
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
                move_player_addicted_weapons,
                update_raygun_rays,
                cleanup_lifetime_over,
                raygun_damage,
                throw_swords,
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
}

/// Mermi tipi - sadece tip belirteci
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProjectileKind {
    Laser { color: Color },
    Rocket { explosion_radius: f32 },
}

// Mermi component'i
#[derive(Component)]
pub struct Projectile {
    pub direction: Vec3,
    pub speed: f32,
    pub damage: f32,
    pub lifetime: Timer,
    pub kind: ProjectileKind,
}

#[derive(Component)]
pub struct Explosion {
    pub lifetime: Timer,
}

#[derive(Component)]
pub struct PlayerAddictedWeapon {
    pub radius: f32,
}

#[derive(Component)]
pub struct RayGunRay {
    pub target_entity: Entity,
    pub lifetime: Timer,
    pub damage: f32,
    pub damage_timer: Timer,
}

impl Default for RayGunRay {
    fn default() -> Self {
        Self {
            target_entity: Entity::PLACEHOLDER,
            lifetime: Timer::from_seconds(0.5, TimerMode::Once),
            damage: 10.0,
            damage_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
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
) {
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

            commands.spawn((
                GameEntity,
                RayGunRay {
                    target_entity: enemy_entity,
                    damage: weapon.damage,
                    ..default()
                },
                Mesh2d(meshes.add(create_thick_line_mesh(Vec3::ZERO, direction, 5.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(raygun.color))),
                Transform::from_translation(player_transform.translation.with_z(10.0)),
                GlobalTransform::default(),
            ));
        }
    }
}
fn create_thick_line_mesh(start: Vec3, end: Vec3, thickness: f32) -> Mesh {
    let direction = (end - start).normalize();
    let perpendicular = Vec3::new(-direction.y, direction.x, 0.0) * thickness * 0.5;

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [
                start.x - perpendicular.x,
                start.y - perpendicular.y,
                start.z,
            ],
            [
                start.x + perpendicular.x,
                start.y + perpendicular.y,
                start.z,
            ],
            [end.x + perpendicular.x, end.y + perpendicular.y, end.z],
            [end.x - perpendicular.x, end.y - perpendicular.y, end.z],
        ],
    );

    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));

    mesh
}

pub fn cleanup_lifetime_over(
    mut commands: Commands,
    time: Res<Time>,
    mut rays: Query<(Entity, &mut RayGunRay), With<RayGunRay>>,
    mut explosions: Query<(Entity, &mut Explosion), With<Explosion>>,
) {
    for (entity, mut ray) in rays.iter_mut() {
        ray.lifetime.tick(time.delta());
        if ray.lifetime.just_finished() {
            commands.entity(entity).despawn();
        }
    }

    for (entity, mut explosion) in explosions.iter_mut() {
        explosion.lifetime.tick(time.delta());
        if explosion.lifetime.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn update_raygun_rays(
    mut raygun_q: Query<
        (&RayGunRay, &mut Transform, &Mesh2d),
        (With<RayGunRay>, Without<Player>, Without<Enemy>),
    >,
    enemies: Query<&Transform, (With<Enemy>, Without<Player>)>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };

    for (ray, mut ray_transform, mesh_handle) in raygun_q.iter_mut() {
        ray_transform.translation = player_transform.translation.with_z(10.0);

        if let Ok(enemy_transform) = enemies.get(ray.target_entity) {
            let direction = enemy_transform.translation - player_transform.translation;

            if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                // Kalın çizgi için mesh'i güncelle
                let perpendicular = Vec3::new(-direction.y, direction.x, 0.0).normalize() * 2.5;

                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    vec![
                        [-perpendicular.x, -perpendicular.y, 0.0],
                        [perpendicular.x, perpendicular.y, 0.0],
                        [
                            direction.x + perpendicular.x,
                            direction.y + perpendicular.y,
                            0.0,
                        ],
                        [
                            direction.x - perpendicular.x,
                            direction.y - perpendicular.y,
                            0.0,
                        ],
                    ],
                );
            }
        }
    }
}

pub fn raygun_damage(
    mut raygun_q: Query<(&mut RayGunRay, Entity), With<RayGunRay>>,
    mut enemies: Query<(Entity, &mut Enemy), (With<Enemy>, Without<Player>)>,
    time: Res<Time>,
    mut commands: Commands,
    mut raygun_weapons: Query<&mut RayGunWeapon, With<RayGunWeapon>>,
) {
    // Ölen düşmanları topla
    let mut dead_enemies = Vec::new();

    // İlk geçiş: damage uygula ve ölen düşmanları topla
    for (mut raygun, raygun_entity) in raygun_q.iter_mut() {
        raygun.damage_timer.tick(time.delta());

        if !raygun.damage_timer.just_finished() {
            continue;
        }

        // Düşman hâlâ var mı kontrol et
        let Ok((enemy_entity, mut enemy)) = enemies.get_mut(raygun.target_entity) else {
            // Düşman yok, ray'i sil
            commands.entity(raygun_entity).despawn();
            continue;
        };

        enemy.health = enemy.health.saturating_sub(raygun.damage as i32);

        if enemy.health <= 0 {
            dead_enemies.push(enemy_entity);
        }
    }

    // İkinci geçiş: ölen düşmanların raylerini temizle
    if !dead_enemies.is_empty() {
        for (raygun, raygun_entity) in raygun_q.iter() {
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
    mut weapons: Query<(&mut Weapon, &LaserWeapon), With<LaserWeapon>>,
    players: Query<&Transform, With<Player>>,
    enemies: Query<&Transform, With<Enemy>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (mut weapon, laser) in weapons.iter_mut() {
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

        // Mermi spawn et
        commands.spawn((
            GameEntity,
            Projectile {
                direction,
                speed: weapon.speed,
                damage: weapon.damage,
                lifetime: Timer::from_seconds(3.0, TimerMode::Once),
                kind: ProjectileKind::Laser { color: laser.color },
            },
            Mesh2d(meshes.add(Circle::new(8.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(laser.color))),
            Transform::from_translation(player_transform.translation + Vec3::new(0.0, 0.0, 10.0)),
            GlobalTransform::default(),
        ));
    }
}

// Roket silahlarını ateşle
pub fn fire_rocket_weapons(
    mut commands: Commands,
    time: Res<Time>,
    mut weapons: Query<(&mut Weapon, &RocketWeapon), With<RocketWeapon>>,
    players: Query<&Transform, With<Player>>,
    enemies: Query<&Transform, With<Enemy>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (mut weapon, rocket) in weapons.iter_mut() {
        weapon.fire_timer.tick(time.delta());

        if !weapon.fire_timer.just_finished() {
            continue;
        }

        let Ok(player_transform) = players.get(weapon.owner) else {
            continue;
        };

        let Some(target_pos) = find_nearest_enemy(player_transform.translation, &enemies) else {
            continue;
        };

        let direction = (target_pos - player_transform.translation).normalize();

        // Roket mermisi spawn et - silah entity'sindeki explosion_radius kullan
        commands.spawn((
            GameEntity,
            Projectile {
                direction,
                speed: weapon.speed,
                damage: weapon.damage,
                lifetime: Timer::from_seconds(5.0, TimerMode::Once),
                kind: ProjectileKind::Rocket {
                    explosion_radius: rocket.explosion_radius,
                },
            },
            Mesh2d(meshes.add(Rectangle::new(12.0, 12.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(1.0, 0.5, 0.0)))),
            Transform::from_translation(player_transform.translation + Vec3::new(0.0, 0.0, 10.0)),
            GlobalTransform::default(),
        ));
    }
}

pub fn move_player_addicted_weapons(
    time: Res<Time>,
    mut player_query: Query<
        (&Transform, &mut Player),
        (
            With<Player>,
            Without<Enemy>,
            Without<Projectile>,
            Without<PlayerAddictedWeapon>,
        ),
    >,
    mut player_addicted_weapon: Query<
        (
            &mut Transform,
            &WeaponStats,
            &mut Weapon,
            &PlayerAddictedWeapon,
            Entity,
        ),
        With<PlayerAddictedWeapon>,
    >,
    mut enemies: Query<(&Transform, &mut Enemy), Without<PlayerAddictedWeapon>>,
) {
    let Ok(player_transform) = player_query.single_mut() else {
        return;
    };
    for (mut addicted_transform, _weapon_stats, mut weapon, addicted_comp, _w_entity) in
        player_addicted_weapon.iter_mut()
    {
        // Pozisyonu takip et
        addicted_transform.translation = player_transform.0.translation;
        // Görsel ölçeği radius'a göre güncelle
        let visual_scale = addicted_comp.radius;
        addicted_transform.scale = Vec3::splat(visual_scale);

        // Ateşleme / hasar mantığı
        weapon.fire_timer.tick(time.delta());
        if !weapon.fire_timer.just_finished() {
            continue;
        }

        let weapon_radius = addicted_comp.radius;
        for (enemy_transform, mut enemy) in enemies.iter_mut() {
            let dist = enemy_transform
                .translation
                .distance(player_transform.0.translation);

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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (proj_entity, mut proj_transform, mut projectile) in projectiles.iter_mut() {
        // Hareketi uygula
        proj_transform.translation += projectile.direction * projectile.speed * time.delta_secs();

        // Ömür kontrolü
        projectile.lifetime.tick(time.delta());
        if projectile.lifetime.just_finished() {
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Düşman çarpışma kontrolü

        match &projectile.kind {
            ProjectileKind::Laser { .. } => {
                for (mut enemy_transform, mut enemy, enemy_aabb) in enemies.iter_mut() {
                    if contains_point(&enemy_aabb, proj_transform.translation) {
                        // Knockback
                        enemy_transform.translation += projectile.direction * 10.;
                        // Hasar
                        enemy.health = enemy.health.saturating_sub(projectile.damage as i32);
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
                    commands.spawn((
                        GameEntity,
                        Mesh2d(meshes.add(Circle::new(*explosion_radius))),
                        MeshMaterial2d(
                            materials.add(ColorMaterial::from(Color::srgba(1.0, 0.1, 0.0, 0.3))),
                        ),
                        Transform::from_translation(explosion_center),
                        Explosion {
                            lifetime: Timer::from_seconds(0.2, TimerMode::Once),
                        },
                    ));

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
    player: Single<(&Player, &Transform), (With<Player>, Without<SwordWeapon>)>,
    mut sword: Query<(&mut SwordWeapon, &mut Weapon), (With<SwordWeapon>, Without<Player>)>,
    window: Single<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    textures: Res<TextureAssets>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let mut direction = Vec3::X;
    let mut is_cursor_at_window = false;

    if let Some(cursor_position) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
            direction = (world_pos - player.1.translation.xy())
                .normalize()
                .extend(0.0);
            is_cursor_at_window = true;
        }
    }
    for (mut sword, mut weapon) in sword.iter_mut() {
        weapon.fire_timer.tick(time.delta());
        if !weapon.fire_timer.just_finished() {
            continue;
        }
        if !is_cursor_at_window {
            direction = sword.last_direction;
        }
        sword.last_direction = direction;
        commands.spawn((
            GameEntity,
            SwordProjectile {
                angle: 0.0,
                direction: direction,
                speed: weapon.speed,
                damage: weapon.damage,
                hit_enemies: Vec::new(),
                lifetime: Timer::from_seconds(2.0, TimerMode::Once),
            },
            Aabb{
                center: player.1.translation.to_vec3a(),
                half_extents: Vec3A::splat(60.0),
            },
            NoFrustumCulling,
            Sprite {
                image: textures.sword.clone(),
                ..default()
            },
            Transform::from_translation(player.1.translation),
        ));
    }
}

pub fn move_swords(
    mut commands: Commands,
    time: Res<Time>,
    mut swords: Query<
        (Entity, &mut Transform, &mut SwordProjectile, &mut Aabb),
        (With<SwordProjectile>, Without<Enemy>),
    >,
    mut enemies: Query<(Entity, &mut Enemy, &mut Aabb, &mut Transform), (With<Enemy>, Without<SwordProjectile>)>,
) {
    for (sword_entity, mut sword_transform, mut sword, mut sword_aabb) in swords.iter_mut() {
        println!("sword position: {:?}", sword_transform.translation);
        // Ömür kontrolü
        sword.lifetime.tick(time.delta());
        if sword.lifetime.just_finished() {
            commands.entity(sword_entity).despawn();
        }

        // Hareket
        sword_transform.translation += sword.direction * sword.speed * time.delta_secs();
        sword_aabb.center = sword_transform.translation.to_vec3a();
        // Döndürme efekti
        sword.angle += 20.0 * time.delta_secs();
        sword_transform.rotation = Quat::from_rotation_z(sword.angle);

        for (enemy_entity, mut enemy, mut aabb, mut transform) in enemies.iter_mut() {
            if aabb_intersects(&aabb, &sword_aabb) && !sword.hit_enemies.contains(&enemy_entity) {
                // Hasar uygula
                enemy.health = enemy.health.saturating_sub(sword.damage as i32);

                transform.translation += sword.direction * 10.0;
                aabb.center = transform.translation.to_vec3a();
                sword.hit_enemies.push(enemy_entity);
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
