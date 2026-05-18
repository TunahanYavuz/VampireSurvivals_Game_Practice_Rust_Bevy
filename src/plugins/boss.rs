use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, NoFrustumCulling};
use bevy::image::TextureAtlas;
use bevy::prelude::*;
use bevy::time::TimerMode;

use crate::plugins::common::GameEntity;
use crate::plugins::config::Config;
use crate::plugins::enemy::Enemy;
use crate::plugins::game::Atlases;
use crate::plugins::game_state::GameState;
use crate::plugins::network::{NetIdCounter, NetworkIdentity, VisualType};
use crate::plugins::player::Player;
use crate::plugins::texture_handling::{TextureAssets, TextureType};
use crate::plugins::timers::GameTimer;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BossSpawnTracker>().add_systems(
            Update,
            (spawn_boss, show_boss_warning).run_if(in_state(GameState::Playing)),
        );
    }
}

// ---------------------------------------------------------------------------
// Component / Resource
// ---------------------------------------------------------------------------

/// Marks an entity as a boss enemy.
#[derive(Component)]
pub struct BossEnemy;

/// Tracks the next boss spawn time (in elapsed game seconds).
#[derive(Resource)]
pub struct BossSpawnTracker {
    pub next_boss_at_secs: f32,
    pub warning_shown: bool,
    pub warning_timer: Timer,
}

impl Default for BossSpawnTracker {
    fn default() -> Self {
        // First boss at 3 minutes (180 s) unless config says otherwise.
        Self {
            next_boss_at_secs: 180.0,
            warning_shown: false,
            warning_timer: Timer::from_seconds(3.0, TimerMode::Once),
        }
    }
}

#[derive(Component)]
struct BossWarningText;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

pub fn spawn_boss(
    mut commands: Commands,
    game_timer: Res<GameTimer>,
    mut tracker: ResMut<BossSpawnTracker>,
    config: Res<Config>,
    atlases: Res<Atlases>,
    textures: Res<TextureAssets>,
    player_query: Query<&Transform, With<Player>>,
    mut net_id_counter: ResMut<NetIdCounter>,
) {
    if game_timer.elapsed_secs < tracker.next_boss_at_secs {
        return;
    }
    if !atlases.ready {
        return;
    }

    let boss_cfg = &config.0.boss;

    // Schedule the next boss
    let interval_secs = boss_cfg.spawn_interval_minutes as f32 * 60.0;
    tracker.next_boss_at_secs += interval_secs;
    tracker.warning_shown = false;

    // Spawn behind the nearest player
    let spawn_pos = if let Ok(pt) = player_query.iter().next().map(Ok).unwrap_or(Err(())) {
        pt.translation + Vec3::new(600.0, 0.0, 0.0)
    } else {
        Vec3::new(600.0, 0.0, 0.0)
    };

    let body_atlas = atlases.body.as_ref().unwrap().clone();

    commands
        .spawn((
            GameEntity,
            BossEnemy,
            NetworkIdentity {
                net_id: net_id_counter.next(),
                visual_type: VisualType::Robot,
            },
            Transform::from_translation(spawn_pos).with_scale(Vec3::splat(3.0)),
            Enemy {
                health: (boss_cfg.health_multiplier * 100.0) as i32,
                speed: 40.0 * boss_cfg.speed_multiplier,
                damage: (boss_cfg.damage_multiplier * 10.0) as i32,
                xp_drop: boss_cfg.xp_drop,
                drops_loot: true,
                base_health: (boss_cfg.health_multiplier * 100.0) as i32,
                base_speed: 40.0,
                base_damage: (boss_cfg.damage_multiplier * 10.0) as i32,
                ..default()
            },
            Aabb {
                center: spawn_pos.into(),
                half_extents: Vec3::new(40.0, 40.0, 0.0).into(),
            },
            NoAutoAabb,
            NoFrustumCulling,
            InheritedVisibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_atlas_image(
                    textures.textures.get(&TextureType::Robot).unwrap().clone(),
                    TextureAtlas {
                        layout: body_atlas,
                        index: 15,
                    },
                ),
                // Visual tint to distinguish boss
                Transform::default(),
            ));
        });
}

/// Show a warning banner ~15 s before the boss arrives, hide after 3 s.
fn show_boss_warning(
    mut commands: Commands,
    game_timer: Res<GameTimer>,
    mut tracker: ResMut<BossSpawnTracker>,
    time: Res<Time>,
    warning_q: Query<Entity, With<BossWarningText>>,
    asset_server: Res<AssetServer>,
    config: Res<Config>,
) {
    let warning_before_secs = 15.0;
    let secs_until_boss = tracker.next_boss_at_secs - game_timer.elapsed_secs;

    // Show warning
    if !tracker.warning_shown && secs_until_boss <= warning_before_secs && secs_until_boss > 0.0 {
        tracker.warning_shown = true;
        tracker.warning_timer.reset();

        let font = asset_server.load("fonts/FiraMono-Medium.ttf");
        let boss_cfg = &config.0.boss;
        commands.spawn((
            GameEntity,
            BossWarningText,
            Text::new(format!(
                "⚠ BOSS INCOMING in {}s! ⚠",
                secs_until_boss.ceil() as u32
            )),
            TextFont {
                font,
                font_size: 44.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.2, 0.0)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(80.0),
                left: Val::Px(300.0),
                ..default()
            },
        ));
        let _ = boss_cfg; // silence unused warning
    }

    // Tick and remove warning
    if tracker.warning_shown {
        tracker.warning_timer.tick(time.delta());
        if tracker.warning_timer.just_finished() {
            for entity in &warning_q {
                commands.entity(entity).try_despawn();
            }
        }
    }
}
