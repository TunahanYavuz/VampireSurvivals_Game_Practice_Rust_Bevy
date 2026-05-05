use crate::plugins::audio::GameAudio;
use crate::plugins::common::{GameEntity, aabb_intersects};
use crate::plugins::game::Atlases;
use crate::plugins::game_state::GameState;
use crate::plugins::network::{NetId, NetIdGenerator, NetworkIdentity, NetworkRole};
use crate::plugins::player::Player;
use crate::plugins::score::GameScore;
use crate::plugins::texture_handling::{TextureAssets, TextureType};
use crate::plugins::timers::{EnemySpawnTimer, GameTimer, MoveTimer};
use bevy::asset::Assets;
use bevy::audio::{AudioPlayer, PlaybackSettings};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, };
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use bevy::mesh::{Mesh};
use bevy::prelude::*;
use bevy::time::TimerMode;
use rand::Rng;
use std::f32::consts::PI;
use crate::plugins::config::Config;
use crate::plugins::reinforcements::spawn_reinforcement;

pub struct EnemyPlugin;

/// Run condition: true when this machine runs the authoritative simulation.
fn is_host_or_solo(role: Res<NetworkRole>) -> bool {
    *role != NetworkRole::Client
}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySpawnTimer>()
            .init_resource::<GameStageManager>()
            .add_systems(
                Update,
                (
                    spawn_enemies,
                    follow,
                    enemy_collision_with_enemy,
                    despawn_enemies,
                    apply_stage_to_existing_enemies,
                )
                    .chain()
                    // Only the host/solo machine runs enemy simulation.
                    .run_if(in_state(GameState::Playing).and(is_host_or_solo)),
            );
    }
}

#[derive(Component)]
pub struct Enemy {
    pub health: i32,
    pub speed: f32,
    pub damage: i32,
    pub xp_drop: i32,
    pub should_despawn: bool,
    pub drops_loot: bool,
    pub base_health: i32,
    pub base_speed: f32,
    pub base_damage: i32,
}

impl Default for Enemy {
    fn default() -> Self {
        Self {
            health: 100,
            speed: 50.0,
            damage: 10,
            xp_drop: 10,
            should_despawn: false,
            drops_loot: true,
            base_health: 100,
            base_speed: 50.0,
            base_damage: 10,
        }
    }
}

#[derive(Resource)]
pub struct GameStageManager {
    pub current_stage_index: usize,
}

impl Default for GameStageManager {
    fn default() -> Self {
        Self {
            current_stage_index: 0,
        }
    }
}

#[derive(Component)]
pub struct XP {
    pub is_collected: bool,
    pub amount: i32,
}

#[derive(Component)]
pub struct Collectible;

#[derive(Component)]
pub struct EnemySprit {
    pub index: usize,
}

fn despawn_enemies(
    mut commands: Commands,
    mut enemy_query: Query<(Entity, &mut Enemy, &Transform), With<Enemy>>,
    mut score: ResMut<GameScore>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    audio: Res<GameAudio>,
) {
    for (enemy_entity, mut enemy, transform) in enemy_query.iter_mut() {
        if enemy.health <= 0 && !enemy.should_despawn {
            enemy.should_despawn = true;
        }

        if !enemy.should_despawn {
            continue;
        }

        commands.spawn((
            AudioPlayer(audio.enemy_hit.clone()),
            PlaybackSettings::DESPAWN,
        ));

        if enemy.drops_loot {
            spawn_reinforcement(
                &mut commands,
                transform.translation,
                enemy.xp_drop,
                &mut meshes,
                &mut materials,
            );
        }

        if let Ok(mut entity_commands) = commands.get_entity(enemy_entity) {
            entity_commands.despawn();
        }
        // Increment the shared score resource instead of mutating a Single player.
        score.score += 1;
    }
}

pub fn follow(
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (&mut Transform, &Enemy, &mut Aabb, &Children),
        (With<Enemy>, Without<Player>),
    >,
    time: Res<Time>,
    mut enemy_move_timer: ResMut<MoveTimer>,
    mut enemy_sprit_query: Query<(&mut Sprite, &mut EnemySprit), With<EnemySprit>>,
) {
    enemy_move_timer.timer.tick(time.delta());
    for (mut enemy_position, enemy, mut aabb, children) in enemy_query.iter_mut() {
        // Target the nearest alive player.
        let Some(target_pos) = player_query
            .iter()
            .map(|t| t.translation)
            .min_by(|a, b| {
                a.distance(enemy_position.translation)
                    .partial_cmp(&b.distance(enemy_position.translation))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        else {
            continue;
        };

        let diff: Vec3 = target_pos - enemy_position.translation;
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
                    if direction.x > 0.0 { 27 + i } else { 9 + i }
                } else {
                    if direction.y > 0.0 { 0 + i } else { 18 + i }
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
    mut stage_manager: ResMut<GameStageManager>,
    game_timer: Res<GameTimer>,
    config: Res<Config>,
    mut net_id_gen: ResMut<NetIdGenerator>,
) {
    let stages = &config.0.stages;
    let enemies = &config.0.enemies;

    // Determine which stage should be active based on elapsed time
    let elapsed_minutes = (game_timer.elapsed_secs / 60.0) as u32;
    let new_stage_index = stages
        .iter()
        .enumerate()
        .filter(|(_, s)| s.minute <= elapsed_minutes)
        .map(|(i, _)| i)
        .last()
        .unwrap_or(0);

    // Update spawn timer if the stage changed
    if new_stage_index != stage_manager.current_stage_index {
        stage_manager.current_stage_index = new_stage_index;
        if let Some(stage) = stages.get(new_stage_index) {
            spawn_timer.timer = Timer::from_seconds(stage.spawn_rate, TimerMode::Repeating);
        }
    }

    spawn_timer.timer.tick(time.delta());
    if !spawn_timer.timer.just_finished() {
        return;
    }
    if !atlases.ready {
        return;
    }

    let Ok(player_transform) = player_query.iter().next().map(Ok::<_, ()>).unwrap_or(Err(())) else {
        return;
    };

    let radius = rand::rng().random_range(500.0..800.0);
    let angle = rand::rng().random_range(0.0..2. * PI);
    let x = player_transform.translation.x + radius * angle.cos();
    let y = player_transform.translation.y + radius * angle.sin();

    let body_atlas = atlases.body.as_ref().unwrap().clone();
    let shield_atlas = atlases.shield.as_ref().unwrap().clone();
    if let Some((body_lay, _shield_lay)) = atlas_layouts
        .get(&body_atlas)
        .zip(atlas_layouts.get(&shield_atlas))
    {
        if let Some(body_rect) = body_lay.textures.get(0) {
            let b_width = body_rect.width() as f32 / 2. - 10.;
            let b_height = body_rect.height() as f32 / 2. - 10.;

            // Get the current stage config (fall back to last stage if beyond all stages)
            let stage = stages
                .get(stage_manager.current_stage_index)
                .or_else(|| stages.last());

            let (enemy_type_index, health_mul, speed_mul, damage_mul) = match stage {
                Some(s) => (s.enemy_type_index, s.health_multiplier, s.speed_multiplier, s.damage_multiplier),
                None => (0, 1.0, 1.0, 1.0),
            };

            let enemy_type_index = enemy_type_index.min(enemies.len().saturating_sub(1));
            let base = &enemies[enemy_type_index];

            let base_health = base.health;
            let base_speed = base.speed;
            let base_damage = base.damage;

            let texture_type = match enemy_type_index {
                0 => TextureType::Zombie,
                1 => TextureType::Knight,
                2 => TextureType::Vampire,
                _ => TextureType::Robot,
            };

            let spirit = Sprite::from_atlas_image(
                textures.textures.get(&texture_type).unwrap().clone(),
                TextureAtlas {
                    layout: body_atlas,
                    index: 15,
                },
            );
            let enemy = Enemy {
                health: (base_health as f32 * health_mul).round() as i32,
                speed: base_speed * speed_mul,
                damage: (base_damage as f32 * damage_mul).round() as i32,
                xp_drop: base.xp_drop,
                should_despawn: false,
                drops_loot: true,
                base_health,
                base_speed,
                base_damage,
            };

            commands
                .spawn((
                    GameEntity,
                    Transform::from_xyz(x, y, 0.0),
                    enemy,
                    InheritedVisibility::default(),
                    Aabb {
                        center: Vec3::new(x, y, 0.0).into(),
                        half_extents: Vec3::new(b_width, b_height, 0.0).into(),
                    },
                    NoAutoAabb,
                    NetworkIdentity(net_id_gen.0),
                ))
                .with_children(|parent| {
                    parent.spawn((spirit, EnemySprit { index: 0 }));

                    parent.spawn((
                        Sprite::from_atlas_image(
                            textures.textures.get(&TextureType::Shield).unwrap().clone(),
                            TextureAtlas {
                                layout: shield_atlas,
                                index: 15,
                            },
                        ),
                        EnemySprit { index: 0 },
                    ));
                });
            net_id_gen.0 += 1;
        }
    }
}

pub fn apply_stage_to_existing_enemies(
    stage_manager: Res<GameStageManager>,
    config: Res<Config>,
    mut enemy_query: Query<&mut Enemy>,
) {
    if !stage_manager.is_changed() {
        return;
    }

    let stages = &config.0.stages;
    let enemies_cfg = &config.0.enemies;

    let stage = stages
        .get(stage_manager.current_stage_index)
        .or_else(|| stages.last());

    let (enemy_type_index, health_mul, speed_mul, damage_mul) = match stage {
        Some(s) => (s.enemy_type_index, s.health_multiplier, s.speed_multiplier, s.damage_multiplier),
        None => return,
    };

    let enemy_type_index = enemy_type_index.min(enemies_cfg.len().saturating_sub(1));
    let base_cfg = &enemies_cfg[enemy_type_index];

    for mut enemy in enemy_query.iter_mut() {
        // Recalculate from stored base values using new multipliers
        let new_max_health = (enemy.base_health as f32 * health_mul).round() as i32;
        let new_speed = enemy.base_speed * speed_mul;
        let new_damage = (enemy.base_damage as f32 * damage_mul).round() as i32;

        // Scale current health proportionally
        let health_ratio = if enemy.health > 0 && new_max_health > 0 {
            enemy.health as f32 / (enemy.base_health as f32 * {
                // find previous multiplier by looking at the previous stage
                if stage_manager.current_stage_index > 0 {
                    stages[stage_manager.current_stage_index - 1].health_multiplier
                } else {
                    1.0
                }
            }).max(1.0)
        } else {
            1.0
        };
        let scaled_health = (new_max_health as f32 * health_ratio.min(1.0)).round() as i32;

        enemy.health = scaled_health.max(1);
        enemy.speed = new_speed;
        enemy.damage = new_damage;
        // Update base fields to match the new enemy type baseline
        enemy.base_health = base_cfg.health;
        enemy.base_speed = base_cfg.speed;
        enemy.base_damage = base_cfg.damage;
    }
}

pub fn enemy_collision_with_enemy(mut enemy_query: Query<(&mut Transform, &Aabb), With<Enemy>>) {
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

