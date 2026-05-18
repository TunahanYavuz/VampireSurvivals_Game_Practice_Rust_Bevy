use crate::plugins::audio::GameAudio;
use crate::plugins::common::{GameEntity, aabb_intersects};
use crate::plugins::config::Config;
use crate::plugins::game::Atlases;
use crate::plugins::game_state::GameState;
use crate::plugins::network::{NetIdCounter, NetworkIdentity, NetworkRole, VisualType};
use crate::plugins::player::Player;
use crate::plugins::reinforcements::spawn_reinforcement;
use crate::plugins::score::GameScore;
use crate::plugins::texture_handling::{TextureAssets, TextureType};
use crate::plugins::timers::{EnemySpawnTimer, GameTimer, MoveTimer};
use bevy::asset::Assets;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoAutoAabb;
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use bevy::prelude::*;
use bevy::time::TimerMode;
use rand::Rng;
use std::f32::consts::PI;

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
                    apply_enemy_scaling,
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
pub struct WaveManager {
    pub current_stage_index: usize,
    pub last_swarm_stage_index: Option<usize>,
}

impl Default for WaveManager {
    fn default() -> Self {
        Self {
            current_stage_index: 0,
            last_swarm_stage_index: None,
        }
    }
}

pub type GameStageManager = WaveManager;

#[derive(Component, Clone, Copy)]
pub struct EnemyScaler {
    pub spawn_time_secs: f32,
    pub stage_health_multiplier: f32,
    pub stage_speed_multiplier: f32,
    pub stage_damage_multiplier: f32,
    pub previous_max_health: f32,
}

#[derive(Component)]
pub struct XP {
    pub is_collected: bool,
    pub amount: i32,
    pub collected_by: Option<u8>,
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
    textures: Res<TextureAssets>,
    _audio: Res<GameAudio>,
    mut net_id_counter: ResMut<NetIdCounter>,
) {
    for (enemy_entity, mut enemy, transform) in enemy_query.iter_mut() {
        if enemy.health <= 0 && !enemy.should_despawn {
            enemy.should_despawn = true;
        }

        if !enemy.should_despawn {
            continue;
        }

        if enemy.drops_loot {
            spawn_reinforcement(
                &mut commands,
                transform.translation,
                enemy.xp_drop,
                &mut net_id_counter,
                &textures,
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
    enemy_move_timer: ResMut<MoveTimer>,
    mut enemy_sprit_query: Query<(&mut Sprite, &mut EnemySprit), With<EnemySprit>>,
) {
    for (mut enemy_position, enemy, mut aabb, children) in enemy_query.iter_mut() {
        // Target the nearest alive player.
        let Some(target_pos) = player_query.iter().map(|t| t.translation).min_by(|a, b| {
            a.distance(enemy_position.translation)
                .partial_cmp(&b.distance(enemy_position.translation))
                .unwrap_or(std::cmp::Ordering::Equal)
        }) else {
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

fn average_player_level(player_query: &Query<&Player>) -> f32 {
    let mut sum = 0.0_f32;
    let mut count = 0.0_f32;
    for player in player_query.iter() {
        sum += player.level.max(1) as f32;
        count += 1.0;
    }
    if count <= 0.0 { 1.0 } else { sum / count }
}

fn dynamic_enemy_scalars(time_alive_secs: f32, average_player_level: f32) -> (f32, f32, f32) {
    let alive_factor = 1.0 + (time_alive_secs * 0.0025).min(0.75);
    let level_factor = 1.0 + ((average_player_level - 1.0).max(0.0) * 0.035);
    let health = alive_factor * level_factor;
    let speed = 1.0 + (alive_factor - 1.0) * 0.35 + (level_factor - 1.0) * 0.25;
    let damage = 1.0 + (alive_factor - 1.0) * 0.60 + (level_factor - 1.0) * 0.50;
    (health, speed, damage)
}

fn resolve_enemy_type_for_stage(
    stage: &crate::plugins::config::StageConfig,
    enemies: &[crate::plugins::config::EnemyConfig],
) -> usize {
    let max_index = enemies.len().saturating_sub(1);
    let primary = stage.enemy_type_index.min(max_index);
    let support = stage.support_enemy_type_index.map(|idx| idx.min(max_index));
    match support {
        Some(support_idx)
            if rand::rng().random::<f32>() < stage.support_enemy_weight.clamp(0.0, 1.0) =>
        {
            support_idx
        }
        _ => primary,
    }
}

fn spawn_enemy_entity(
    commands: &mut Commands,
    atlases: &Atlases,
    atlas_layouts: &Assets<TextureAtlasLayout>,
    textures: &TextureAssets,
    net_id_counter: &mut NetIdCounter,
    enemies_cfg: &[crate::plugins::config::EnemyConfig],
    stage: &crate::plugins::config::StageConfig,
    spawn_pos: Vec3,
    game_timer: &GameTimer,
    average_player_level: f32,
) {
    let Some(body_atlas) = atlases.body.as_ref().cloned() else {
        return;
    };
    let Some(shield_atlas) = atlases.shield.as_ref().cloned() else {
        return;
    };
    let Some((body_lay, _shield_lay)) = atlas_layouts
        .get(&body_atlas)
        .zip(atlas_layouts.get(&shield_atlas))
    else {
        return;
    };
    let Some(body_rect) = body_lay.textures.get(0) else {
        return;
    };

    let b_width = body_rect.width() as f32 / 2.0 - 10.0;
    let b_height = body_rect.height() as f32 / 2.0 - 10.0;

    let enemy_type_index = resolve_enemy_type_for_stage(stage, enemies_cfg);
    let base = &enemies_cfg[enemy_type_index];
    let (dyn_h, dyn_s, dyn_d) = dynamic_enemy_scalars(0.0, average_player_level);
    let stage_h = stage.health_multiplier;
    let stage_s = stage.speed_multiplier;
    let stage_d = stage.damage_multiplier;

    let max_health = (base.health as f32 * stage_h * dyn_h).round().max(1.0);
    let speed = (base.speed * stage_s * dyn_s).max(25.0);
    let damage = (base.damage as f32 * stage_d * dyn_d).round().max(1.0);

    let texture_type = match enemy_type_index {
        0 => TextureType::Zombie,
        1 => TextureType::Knight,
        2 => TextureType::Vampire,
        _ => TextureType::Robot,
    };

    let visual_type = match enemy_type_index {
        0 => VisualType::Zombie,
        1 => VisualType::Knight,
        2 => VisualType::Vampire,
        _ => VisualType::Robot,
    };

    let spirit = Sprite::from_atlas_image(
        textures.textures.get(&texture_type).unwrap().clone(),
        TextureAtlas {
            layout: body_atlas,
            index: 15,
        },
    );

    commands
        .spawn((
            GameEntity,
            NetworkIdentity {
                net_id: net_id_counter.next(),
                visual_type,
            },
            Transform::from_translation(spawn_pos),
            Enemy {
                health: max_health as i32,
                speed,
                damage: damage as i32,
                xp_drop: base.xp_drop,
                should_despawn: false,
                drops_loot: true,
                base_health: base.health,
                base_speed: base.speed,
                base_damage: base.damage,
            },
            EnemyScaler {
                spawn_time_secs: game_timer.elapsed_secs,
                stage_health_multiplier: stage_h,
                stage_speed_multiplier: stage_s,
                stage_damage_multiplier: stage_d,
                previous_max_health: max_health,
            },
            InheritedVisibility::default(),
            Aabb {
                center: spawn_pos.to_vec3a(),
                half_extents: Vec3::new(b_width, b_height, 0.0).into(),
            },
            NoAutoAabb,
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
    mut net_id_counter: ResMut<NetIdCounter>,
    player_level_query: Query<&Player>,
) {
    let stages = &config.0.stages;
    let enemies = &config.0.enemies;
    if stages.is_empty() || enemies.is_empty() || !atlases.ready {
        return;
    }

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
            let interval = (1.0 / stage.enemies_per_second()).max(0.03);
            spawn_timer.timer = Timer::from_seconds(interval, TimerMode::Repeating);
        }
    }

    let Ok(player_transform) = player_query
        .iter()
        .next()
        .map(Ok::<_, ()>)
        .unwrap_or(Err(()))
    else {
        return;
    };
    let stage = stages
        .get(stage_manager.current_stage_index)
        .or_else(|| stages.last())
        .unwrap();
    let avg_level = average_player_level(&player_level_query);

    if stage.swarm_size > 0
        && stage_manager.last_swarm_stage_index != Some(stage_manager.current_stage_index)
    {
        stage_manager.last_swarm_stage_index = Some(stage_manager.current_stage_index);
        for _ in 0..stage.swarm_size {
            let radius = rand::rng().random_range(520.0..780.0);
            let angle = rand::rng().random_range(0.0..2.0 * PI);
            let spawn_pos = Vec3::new(
                player_transform.translation.x + radius * angle.cos(),
                player_transform.translation.y + radius * angle.sin(),
                0.0,
            );
            spawn_enemy_entity(
                &mut commands,
                &atlases,
                &atlas_layouts,
                &textures,
                &mut net_id_counter,
                enemies,
                stage,
                spawn_pos,
                &game_timer,
                avg_level,
            );
        }
    }

    spawn_timer.timer.tick(time.delta());
    if !spawn_timer.timer.just_finished() {
        return;
    }

    let radius = rand::rng().random_range(500.0..800.0);
    let angle = rand::rng().random_range(0.0..2.0 * PI);
    let spawn_pos = Vec3::new(
        player_transform.translation.x + radius * angle.cos(),
        player_transform.translation.y + radius * angle.sin(),
        0.0,
    );
    spawn_enemy_entity(
        &mut commands,
        &atlases,
        &atlas_layouts,
        &textures,
        &mut net_id_counter,
        enemies,
        stage,
        spawn_pos,
        &game_timer,
        avg_level,
    );
}

pub fn apply_stage_to_existing_enemies(
    stage_manager: Res<GameStageManager>,
    config: Res<Config>,
    mut enemy_query: Query<(&mut Enemy, &mut EnemyScaler)>,
) {
    if !stage_manager.is_changed() {
        return;
    }

    let stages = &config.0.stages;
    let stage = stages
        .get(stage_manager.current_stage_index)
        .or_else(|| stages.last());

    let (health_mul, speed_mul, damage_mul) = match stage {
        Some(s) => (s.health_multiplier, s.speed_multiplier, s.damage_multiplier),
        None => return,
    };

    for (_, mut scaler) in enemy_query.iter_mut() {
        scaler.stage_health_multiplier = health_mul;
        scaler.stage_speed_multiplier = speed_mul;
        scaler.stage_damage_multiplier = damage_mul;
    }
}

pub fn apply_enemy_scaling(
    game_timer: Res<GameTimer>,
    player_query: Query<&Player>,
    mut enemies: Query<(&mut Enemy, &mut EnemyScaler)>,
) {
    let avg_level = average_player_level(&player_query);
    for (mut enemy, mut scaler) in enemies.iter_mut() {
        let time_alive = (game_timer.elapsed_secs - scaler.spawn_time_secs).max(0.0);
        let (dyn_h, dyn_s, dyn_d) = dynamic_enemy_scalars(time_alive, avg_level);

        let new_max_health =
            (enemy.base_health as f32 * scaler.stage_health_multiplier * dyn_h).max(1.0);
        let health_ratio = if scaler.previous_max_health > 0.0 {
            (enemy.health as f32 / scaler.previous_max_health).clamp(0.0, 1.0)
        } else {
            1.0
        };
        enemy.health = (new_max_health * health_ratio).round().max(1.0) as i32;
        enemy.speed = (enemy.base_speed * scaler.stage_speed_multiplier * dyn_s).max(25.0);
        enemy.damage = (enemy.base_damage as f32 * scaler.stage_damage_multiplier * dyn_d)
            .round()
            .max(1.0) as i32;
        scaler.previous_max_health = new_max_health;
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
