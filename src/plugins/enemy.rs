use std::f32::consts::PI;
use bevy::asset::{Assets,};
use bevy::audio::{AudioPlayer, PlaybackSettings};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, NoFrustumCulling};
use bevy::color::Color;
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use bevy::mesh::{Mesh, Mesh2d};
use bevy::prelude::*;
use bevy::time::TimerMode;
use rand::Rng;
use crate::plugins::audio::GameAudio;
use crate::plugins::common::{aabb_intersects, GameEntity};
use crate::plugins::game::Atlases;
use crate::plugins::game_state::GameState;
use crate::plugins::player::Player;
use crate::plugins::texture_handling::TextureAssets;
use crate::plugins::timers::{EnemySpawnTimer, MoveTimer};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EnemySpawnTimer>()
            .init_resource::<EnemyPowerUpTimer>()
            .add_systems(
                Update,
                (
                    spawn_enemies,
                    despawn_enemies,
                    follow,
                    enemy_collision_with_enemy,
                ).run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Enemy {
    pub health: i32,
    pub speed: f32,
    pub damage: i32,
    pub xp_drop: i32,
}
#[derive(Resource)]
pub struct EnemyPowerUpTimer {
    pub timer: Timer,
    pub level: i32,
}

#[derive(Component)]
#[derive(Debug)]
pub struct XP{
    pub amount: i32,
}

#[derive(Component)]
pub struct Collectible;

impl Default for EnemyPowerUpTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(40.0, TimerMode::Repeating),
            level: 1,
        }
    }
}

#[derive(Component)]
pub struct EnemySprit {
    pub index: usize,
}


fn despawn_enemies(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Enemy, &Transform), With<Enemy>>,
    mut player: Single<&mut Player>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    audio: Res<GameAudio>,
) {

    for (enemy_entity, enemy, transform) in enemy_query.iter() {
        if enemy.health > 0 {
            continue;
        }

        commands.spawn((
            GameEntity,
            Collectible,
            XP{ amount: enemy.xp_drop },
            Transform::from_translation(transform.translation),
            // AABB{
            //     max_x: translation.x + 20.,
            //     max_y: translation.y + 20.,
            //     min_x: translation.x - 20.,
            //     min_y: translation.y - 20.,
            //     width: 20.,
            //     height: 20.,
            // },
            Aabb{
                center: transform.translation.to_vec3a(),
                half_extents: Vec3A::new(40.0, 40.0, 1.0),
            },
            NoAutoAabb,
            NoFrustumCulling,
            Mesh2d(meshes.add(Circle::new(5.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.8, 0.0, 0.0)))),
        ));
        commands.spawn((
            AudioPlayer(audio.enemy_hit.clone()),
            PlaybackSettings::DESPAWN,
        ));

        commands.entity(enemy_entity).try_despawn();
        player.score += 1;

    }
}


pub fn follow(
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<(&mut Transform, &Enemy, &mut Aabb, &Children), (With<Enemy>, Without<Player>)>,
    time: Res<Time>,
    mut enemy_move_timer: ResMut<MoveTimer>,
    mut enemy_sprit_query: Query<(&mut Sprite, &mut EnemySprit), With<EnemySprit>>,

) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_position = player_transform.translation;

    enemy_move_timer.timer.tick(time.delta());
    for (mut enemy_position, enemy, mut aabb, children) in enemy_query.iter_mut(){
        let diff: Vec3  = player_position - enemy_position.translation;
        if diff.length_squared() < 1e-6 {
            continue;
        }
        let direction = diff.normalize();
        enemy_position.translation += direction * enemy.speed * time.delta_secs();
        aabb.center = enemy_position.translation.to_vec3a();
        if !enemy_move_timer.timer.just_finished() {
            continue;
        }

        for child in children.iter() {
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
    atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    textures: Res<TextureAssets>,
    mut enemy_power: ResMut<EnemyPowerUpTimer>,
) {
    enemy_power.timer.tick(time.delta());
    if enemy_power.timer.just_finished() {
        enemy_power.level += 1;
    }
    let level = enemy_power.level;
    
    spawn_timer.timer.tick(time.delta());
    if !spawn_timer.timer.just_finished() { return; }
    if !atlases.ready { return; }

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let radius = rand::rng().random_range(500.0..800.0);
    let angle = rand::rng().random_range(0.0..2. * PI);
    let x = player_transform.translation.x + radius * angle.cos();
    let y = player_transform.translation.y + radius * angle.sin();

    let body_atlas = atlases.body.as_ref().unwrap().clone();
    let shield_atlas = atlases.shield.as_ref().unwrap().clone();
    if let Some((body_lay, _shield_lay)) = atlas_layouts.get(&body_atlas).zip(atlas_layouts.get(&shield_atlas)) {
        if let Some(body_rect) = body_lay.textures.get(0) {
            let b_width = body_rect.width() as f32 /2. - 10.;
            let b_height = body_rect.height() as f32 /2. - 10.;


            let spirit = match level {
                1 => Sprite::from_atlas_image(textures.robot.clone(), TextureAtlas { layout: body_atlas, index: 15 }),
                2 => Sprite::from_atlas_image(textures.wizard.clone(), TextureAtlas { layout: body_atlas, index: 15 }),
                _ => Sprite::from_atlas_image(textures.elf.clone(), TextureAtlas { layout: body_atlas, index: 15 }),
            };


            commands
                .spawn((
                    GameEntity,
                    Transform::from_xyz(x, y, 0.0),
                    Enemy {
                        health: 100 * level, damage: 1 * level,
                        speed: rand::rng().random_range((100.0 * level as f32) ..200.0 * level as f32),
                        xp_drop: 20 * level},
                    InheritedVisibility::default(),
                    Aabb{
                        center: Vec3::new(x, y, 0.0).into(),
                        half_extents: Vec3::new(b_width, b_height, 0.0).into(),
                    },
                    NoAutoAabb,
                    //AABB { max_x: x + b_width, max_y: y + b_height, min_x: x - b_width, min_y: y - b_height, width: b_width * 2., height: b_height * 2. },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        spirit,
                        EnemySprit { index: 0 },
                    ));

                    parent.spawn((
                        Sprite::from_atlas_image(textures.shield.clone(), TextureAtlas { layout: shield_atlas, index: 15 }),
                        EnemySprit { index: 0 },
                    ));
                });
        }
    }


}

pub fn enemy_collision_with_enemy(
    mut enemy_query: Query<(&mut Transform, &Aabb), With<Enemy>>,
){
    let mut combinations = enemy_query.iter_combinations_mut();

    while let Some([(mut transform1, aabb1), (mut transform2, aabb2)]) = combinations.fetch_next() {
        if aabb_intersects(aabb1, aabb2) {
            let direction = (transform1.translation - transform2.translation).normalize();
            let push_strength = 2.0;

            transform1.translation += direction * push_strength;
            transform2.translation -= direction * push_strength;
        }
    }
}
