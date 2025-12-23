use bevy::prelude::{Children, Component, Entity, Handle, Query, Sprite, TextureAtlasLayout, Time, Transform, Vec3, With};
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::prelude::Without;
use crate::plugins::aabb::AABB;
use crate::plugins::player::Player;
use crate::plugins::texture_handling::TextureAssets;
use crate::plugins::timers::EnemyMoveTimer;

#[derive(Component)]
pub struct Enemy {
    pub health: i32,
    pub speed: f32,
    pub damage: i32,
}

#[derive(Component)]
pub struct EnemySprit {
    pub index: usize,
}

pub fn follow(
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<(&mut Transform, &Enemy, &Children, &mut AABB), (With<Enemy>, Without<Player>)>,
    time: Res<Time>,
    mut enemy_move_timer: ResMut<EnemyMoveTimer>,
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