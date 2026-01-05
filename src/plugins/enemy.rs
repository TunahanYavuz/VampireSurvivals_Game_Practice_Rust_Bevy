use bevy::asset::Assets;
use bevy::color::Color;
use bevy::image::TextureAtlas;
use bevy::mesh::{Mesh, Mesh2d};
use bevy::prelude::{Children, Circle, ColorMaterial, Component, InheritedVisibility, MeshMaterial2d, NextState, Query, Resource, Sprite, States, Time, Timer, Transform, Vec3, With};
use bevy::time::TimerMode;
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Commands, Without};
use rand::Rng;
use crate::Atlases;
use crate::plugins::aabb::AABB;
use crate::plugins::player::Player;
use crate::plugins::texture_handling::TextureAssets;
use crate::plugins::timers::{EnemySpawnTimer, MoveTimer};
use crate::plugins::weapons::GameEntity;

#[derive(Component)]
pub struct Enemy {
    pub health: i32,
    pub speed: f32,
    pub damage: i32,
}
#[derive(Resource)]
pub struct EnemyPowerUpTimer {
    pub timer: Timer,
    pub level: i32,
}

#[derive(Component)]
pub struct XP{
    pub amount: i32,
}

#[derive(Component)]
pub struct Collectible;

impl Default for EnemyPowerUpTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(10.0, TimerMode::Repeating),
            level: 1,
        }
    }
}

impl Enemy {
    pub fn despawn(&mut self, entity: Entity, translation: &Vec3, meshes: &mut Assets<Mesh>, mesh_material: &mut Assets<ColorMaterial>, commands: &mut Commands) {
        commands.spawn((
            Collectible,
            XP{ amount: 20 },
            Transform::from_translation(*translation),
            AABB{
                max_x: translation.x + 2.,
                max_y: translation.y + 2.,
                min_x: translation.x - 2.,
                min_y: translation.y - 2.,
                width: 7.,
                height: 7.,
            },
            Mesh2d(meshes.add(Circle::new(5.0))),
            MeshMaterial2d(mesh_material.add(ColorMaterial::from(Color::srgb(0.8, 0.0, 0.0)))),
            ));
        commands.entity(entity).try_despawn();
    }
}

#[derive(Component)]
pub struct EnemySprit {
    pub index: usize,
}



pub fn follow(
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<(&mut Transform, &Enemy, &Children, &mut AABB), (With<Enemy>, Without<Player>)>,
    time: Res<Time>,
    mut enemy_move_timer: ResMut<MoveTimer>,
    mut enemy_sprit_query: Query<(&mut Sprite, &mut EnemySprit), With<EnemySprit>>,

) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_position = player_transform.translation;

    enemy_move_timer.timer.tick(time.delta());
    for (mut enemy_position, enemy, children, mut aabb) in enemy_query.iter_mut(){
        let diff: Vec3  = player_position - enemy_position.translation;
        if diff.length_squared() < 1e-6 {
            continue;
        }
        let direction = diff.normalize();
        enemy_position.translation += direction * enemy.speed * time.delta_secs();
        aabb.change_point(enemy_position.translation);

        if !enemy_move_timer.timer.just_finished() {
            continue;
        }

        for &child in children.iter() {
            if let Ok((mut sprite, mut enemy_sprit)) = enemy_sprit_query.get_mut(child) {
                let i = (enemy_sprit.index + 1) % 9;
                enemy_sprit.index = i;

                let atlas_index = if direction.x.abs() > direction.y.abs() {
                    if direction.x > 0.0 {
                        27 + i
                    } else {
                        9 + i
                    }
                } else {
                    if direction.y > 0.0 {
                        0 + i
                    } else {
                        18 + i
                    }
                };

                if let Some(ref mut atlas) = sprite.texture_atlas {
                    atlas.index = atlas_index;
                }
            }
        }
    }
}
pub fn spawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<EnemySpawnTimer>,
    player_query: Query<&Transform, With<Player>>,
    atlases: Res<Atlases>,
    textures: Res<TextureAssets>,
    mut enemy_power: ResMut<EnemyPowerUpTimer>
) {
    enemy_power.timer.tick(time.delta());
    if enemy_power.timer.just_finished() {
        enemy_power.level += 1;
    }
    let level = enemy_power.level;
    
    spawn_timer.timer.tick(time.delta());
    if !spawn_timer.timer.just_finished() { return; }
    if !atlases.ready { return; }

    // Query'den güvenli bir şekilde al
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let nx: f32 = rand::rng().random_range(-500.0 - player_transform.translation.x..-200.0 - player_transform.translation.x);
    let ny: f32 = rand::rng().random_range(-500.0 - player_transform.translation.y..-200.0 - player_transform.translation.y);
    let px: f32 = rand::rng().random_range(200.0 + player_transform.translation.x..500.0 + player_transform.translation.x);
    let py: f32 = rand::rng().random_range(200.0 + player_transform.translation.y..500.0 + player_transform.translation.y);
    let x = if nx.abs() > px.abs() { nx } else { px };
    let y = if ny.abs() > py.abs() { ny } else { py };

    let body_atlas = atlases.body.as_ref().unwrap().clone();
    let shield_atlas = atlases.shield.as_ref().unwrap().clone();

    commands
        .spawn((
            GameEntity,  // ← Marker eklendi
            Transform::from_xyz(x, y, 0.0),
            Enemy { health: 100 * level, damage: 1 * level, speed: rand::rng().random_range((100.0 * level as f32) ..200.0 * level as f32) },
            InheritedVisibility::default(),
            AABB { max_x: x + 25., max_y: y + 25., min_x: x - 25., min_y: y - 25., width: 50., height: 50. },
        ))
        .with_children(|parent| {
            parent.spawn((
                GameEntity,  // ← Child'lara da marker
                Sprite::from_atlas_image(textures.body.clone(), TextureAtlas { layout: body_atlas.clone(), index: 15 }),
                EnemySprit { index: 0 },
            ));
            parent.spawn((
                GameEntity,  // ← Child'lara da marker
                Sprite::from_atlas_image(textures.shield.clone(), TextureAtlas { layout: shield_atlas.clone(), index: 15 }),
                EnemySprit { index: 0 },
            ));
        });
}