use bevy::prelude::*;
use crate::plugins::aabb::AABB;
use crate::plugins::audio::GameAudio;
use crate::plugins::enemy::Enemy;
use crate::plugins::player::Player;
use crate::plugins::weapon_stats::WeaponStats;

// GameEntity marker
#[derive(Component)]
pub struct GameEntity;

// Temel silah component'i
#[derive(Component)]
pub struct Weapon {
    pub owner: Entity,
    pub damage: f32,
    pub fire_timer: Timer,
    pub speed: f32,
}

// Farklı silah tipleri
// Farklı silah tipleri - sadece özellikler
#[derive(Component, Clone, Copy, PartialEq)]
pub struct LaserWeapon {
    pub color: Color,
}

#[derive(Component, Clone, Copy, PartialEq)]
pub struct RocketWeapon {
    pub explosion_radius: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProjectileKind{
    Laser{lazer_weapon: LaserWeapon},
    Rocket{rocket_weapon: RocketWeapon},
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
pub struct Explosion{
    pub lifetime: Timer,
}

#[derive(Component)]
pub struct PlayerAddictedWeapon{
    pub radius: f32,
}



// Player için silahları bir kere spawn et


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
        
        // Mermi spawn et (ColorMaterial::from kullanıldı)
        commands.spawn((
            GameEntity,
            Projectile {
                direction,
                speed: weapon.speed,
                damage: weapon.damage,
                lifetime: Timer::from_seconds(3.0, TimerMode::Once),
                kind: ProjectileKind::Laser {lazer_weapon: *laser },
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
    mut weapons: Query<(&mut Weapon, &WeaponStats, &RocketWeapon), With<RocketWeapon>>,
    players: Query<&Transform, With<Player>>,
    enemies: Query<&Transform, With<Enemy>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (mut weapon, rocket_stats, rocket_comp) in weapons.iter_mut() {
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

        // Roket mermisi spawn et (ColorMaterial::from kullanıldı)
        commands.spawn((
            GameEntity,
            Projectile {
                direction,
                speed: rocket_stats.base_speed,
                damage: weapon.damage,
                lifetime: Timer::from_seconds(5.0, TimerMode::Once),
                kind: ProjectileKind::Rocket { rocket_weapon: *rocket_comp },
            },
            Mesh2d(meshes.add(Rectangle::new(12.0, 12.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(1.0, 0.5, 0.0)))),
            Transform::from_translation(player_transform.translation + Vec3::new(0.0, 0.0, 10.0)),
            GlobalTransform::default(),
        ));
    }
}



pub fn move_player_addicted_weapons(
    mut commands: Commands,
    time: Res<Time>,
    mut player_query: Query<(&Transform, &mut Player), (With<Player>, Without<Enemy>, Without<Projectile>, Without<PlayerAddictedWeapon>)>,
    // PlayerAddictedWeapon referansını da alıyoruz ki radius'ı okuyup görseli güncelleyebilelim
    mut player_addicted_weapon: Query<(&mut Transform, &WeaponStats, &mut Weapon, &PlayerAddictedWeapon), With<PlayerAddictedWeapon>>,
    mut enemies: Query<(&Transform, Entity, &mut Enemy), Without<PlayerAddictedWeapon>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    audio: Res<GameAudio>

){
    let Ok(mut player_transform) = player_query.single_mut() else { return; };
    for (mut addicted_transform, _weapon_stats, mut weapon, addicted_comp) in player_addicted_weapon.iter_mut() {
        // Pozisyonu takip et
        addicted_transform.translation = player_transform.0.translation;
        // Görsel ölçeği radius'a göre güncelle
        let visual_scale = addicted_comp.radius;
        addicted_transform.scale = Vec3::splat(visual_scale);

        // Ateşleme / hasar mantığı
        weapon.fire_timer.tick(time.delta());
        if !weapon.fire_timer.just_finished() { continue; }

        let weapon_radius = addicted_comp.radius;
        for (enemy_transform, enemy_entity, mut enemy) in enemies.iter_mut() {
            let dist = enemy_transform.translation.distance(player_transform.0.translation);

            if dist <= weapon_radius {
                enemy.health = enemy.health.saturating_sub(weapon.damage as i32);
                if enemy.health <= 0 {
                    player_transform.1.score += 1;
                    enemy.despawn(enemy_entity, &enemy_transform.translation, &mut *meshes, &mut *materials, &mut commands, &audio);
                }
            }
        }
    }
}

pub fn despawn_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut explosions: Query<(Entity, &mut Explosion), With<Explosion>>,
){
    for (explosion_entity, mut explosion) in explosions.iter_mut() {
        explosion.lifetime.tick(time.delta());
        if explosion.lifetime.just_finished() {
            commands.entity(explosion_entity).despawn();
        }
    }
}

// Mermileri hareket ettir ve çarpışma kontrolü yap
pub fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile), With<Projectile>>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy, &mut AABB), Without<Projectile>>,
    mut player: Single<&mut Player>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    audio: Res<GameAudio>
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
        let mut rocket_exploded = false;

        // Düşman çarpışma kontrolü


            match &projectile.kind {
                ProjectileKind::Laser { .. } => {
                    for (enemy_entity, mut enemy_transform, mut enemy, mut enemy_aabb) in enemies.iter_mut() {
                        if enemy_aabb.contains_point(proj_transform.translation) {
                            // Knockback
                            enemy_transform.translation += projectile.direction * 10.;
                            enemy_aabb.change_point(enemy_transform.translation);
                            // Hasar
                            enemy.health = enemy.health.saturating_sub(projectile.damage as i32);
                            // Mermi yok et
                            commands.entity(proj_entity).try_despawn();
                            // Düşman öldüyse
                            if enemy.health <= 0 {
                                enemy.despawn(enemy_entity, &enemy_transform.translation, &mut *meshes, &mut *materials, &mut commands, &audio);
                                player.score += 1;
                            }
                            break;
                        }
                    }
                }
                ProjectileKind::Rocket { rocket_weapon } => {
                    // Önce roketin herhangi bir düşmana çarpıp çarpmadığını kontrol et
                    let mut explosion_pos: Option<Vec3> = None;

                    for (_enemy_entity, _enemy_transform, _enemy, enemy_aabb) in enemies.iter() {
                        if enemy_aabb.contains_point(proj_transform.translation) {
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
                            Mesh2d(meshes.add(Circle::new(rocket_weapon.explosion_radius))),
                            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(1.0, 0.1, 0.0, 0.3)))),
                            Transform::from_translation(explosion_center),
                            Explosion {
                                lifetime: Timer::from_seconds(0.2, TimerMode::Once),
                            },
                        ));

                        // Tüm düşmanları tekrar tara ve patlama yarıçapındakilere hasar ver
                        for (enemy_entity, mut enemy_transform, mut enemy, mut enemy_aabb) in enemies.iter_mut() {
                            let dist = enemy_transform.translation.distance(explosion_center);
                            if dist <= rocket_weapon.explosion_radius {
                                // Knockback - patlamadan uzağa it
                                let knockback_dir = (enemy_transform.translation - explosion_center).normalize_or_zero();
                                enemy_transform.translation += knockback_dir * 20.;
                                enemy_aabb.change_point(enemy_transform.translation);

                                // Hasar
                                enemy.health = enemy.health.saturating_sub(projectile.damage as i32);
                                if enemy.health <= 0 {
                                    enemy.despawn(enemy_entity, &enemy_transform.translation, &mut *meshes, &mut *materials, &mut commands, &audio);
                                    player.score += 1;
                                }
                            }
                        }

                        // Roketi sil
                        commands.entity(proj_entity).try_despawn();
                    }
                }
            }


    }
}

// Yardımcı fonksiyon - en yakın düşmanı bul
fn find_nearest_enemy(
    position: Vec3,
    enemies: &Query<&Transform, With<Enemy>>
) -> Option<Vec3> {
    enemies
        .iter()
        .map(|t| (t.translation, position.distance(t.translation)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(pos, _)| pos)
}
