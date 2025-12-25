use bevy::prelude::*;
use crate::plugins::aabb::AABB;
use crate::plugins::enemy::Enemy;
use crate::plugins::player::Player;
use crate::plugins::score::GameScore;
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
}

// Farklı silah tipleri
// Farklı silah tipleri - sadece özellikler
#[derive(Component)]
pub struct LaserWeapon {
    pub speed: f32,
    pub color: Color,
}

#[derive(Component)]
pub struct RocketWeapon {
    pub speed: f32,
    pub explosion_radius: f32,
}

#[derive(Component)]
pub struct FlameWeapon {
    pub range: f32,
}


// Mermi component'i
#[derive(Component)]
pub struct Projectile {
    pub direction: Vec3,
    pub speed: f32,
    pub damage: f32,
    pub lifetime: Timer,
}

#[derive(Component)]
pub struct PlayerAddictedWeapon{
    pub damage: f32,
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
        
        // Mermi spawn et
        commands.spawn((
            GameEntity,
            Projectile {
                direction,
                speed: laser.speed,
                damage: weapon.damage,
                lifetime: Timer::from_seconds(3.0, TimerMode::Once),
            },
            Mesh2d(meshes.add(Circle::new(8.0))),
            MeshMaterial2d(materials.add(laser.color)),
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
        
        // Roket mermisi spawn et
        commands.spawn((
            GameEntity,
            Projectile {
                direction,
                speed: rocket.speed,
                damage: weapon.damage,
                lifetime: Timer::from_seconds(5.0, TimerMode::Once),
            },
            Mesh2d(meshes.add(Rectangle::new(12.0, 12.0))),
            MeshMaterial2d(materials.add(Color::srgb(1.0, 0.5, 0.0))),
            Transform::from_translation(player_transform.translation + Vec3::new(0.0, 0.0, 10.0)),
            GlobalTransform::default(),
        ));
    }
}



pub fn move_player_addicted_weapons(
    mut commands: Commands,
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>, Without<Projectile>, Without<PlayerAddictedWeapon>)>,
    mut player_addicted_weapon: Query<(&mut Transform, &WeaponStats, &mut Weapon), With<PlayerAddictedWeapon>>,
    mut enemies: Query<(&Transform, Entity, &mut Enemy), Without<PlayerAddictedWeapon>>,
    mut score: ResMut<GameScore>,
){
    let Ok(player_transform) = player_query.single() else { return; };
    for (mut addicted_transform, weapon_stats, mut weapon) in player_addicted_weapon.iter_mut() {
        addicted_transform.translation = player_transform.translation;
        weapon.fire_timer.tick(time.delta());
        if !weapon.fire_timer.just_finished() { continue; }
        let weapon_radius = weapon_stats.base_range * (addicted_transform.scale.x.abs());
        for (enemy_transform, enemy_entity, mut enemy) in enemies.iter_mut() {
            let dist = enemy_transform.translation.distance(player_transform.translation);

            if dist <= weapon_radius {
                enemy.health = enemy.health.saturating_sub(weapon.damage as i32);
                println!("{}", weapon.damage);
                if enemy.health <=0 {
                    score.score += 1;
                    commands.entity(enemy_entity).try_despawn();
                }
            }
        }
    }
}

// Mermileri hareket ettir ve çarpışma kontrolü yap
pub fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile), With<Projectile>>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy, &mut AABB), Without<Projectile>>,
    mut score: ResMut<GameScore>,
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
        for (enemy_entity, mut enemy_transform, mut enemy, mut enemy_aabb) in enemies.iter_mut() {
            if enemy_aabb.contains_point(proj_transform.translation) {
                enemy_transform.translation += projectile.direction * 10.;
                enemy_aabb.change_point(enemy_transform.translation);
                // Hasar ver
                enemy.health = enemy.health.saturating_sub(projectile.damage as i32);
                // Mermiyi yok et
                commands.entity(proj_entity).despawn();
                
                // Düşman öldüyse
                if enemy.health <= 0 {
                    commands.entity(enemy_entity).try_despawn();
                    score.score += 1;
                }
                break;
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


