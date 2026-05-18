use serde::{Deserialize, Serialize};

use bevy::prelude::*;
use bevy::window::WindowResolution;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct GameConfig {
    pub window: WindowConfig,
    pub player: PlayerConfig,
    pub enemies: Vec<EnemyConfig>,
    pub stages: Vec<StageConfig>,
    pub boss: BossConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub resizable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerConfig {
    pub speed: f32,
    pub health: u32,
    pub max_health: u32,
    pub starting_weapon: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnemyConfig {
    pub name: String,
    pub health: i32,
    pub speed: f32,
    pub damage: i32,
    pub spawn_rate: f32,
    pub xp_drop: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StageConfig {
    pub minute: u32,
    pub health_multiplier: f32,
    pub speed_multiplier: f32,
    pub damage_multiplier: f32,
    /// Legacy field: seconds between enemy spawns.
    #[serde(default)]
    pub spawn_rate: f32,
    /// Preferred field: enemies spawned per second.
    #[serde(default = "default_spawn_per_second")]
    pub spawn_per_second: f32,
    pub enemy_type_index: usize,
    /// Optional secondary enemy type mixed into this stage.
    #[serde(default)]
    pub support_enemy_type_index: Option<usize>,
    /// Chance [0..1] to spawn support enemy instead of primary.
    #[serde(default)]
    pub support_enemy_weight: f32,
    /// One-time burst size triggered when entering this stage.
    #[serde(default)]
    pub swarm_size: u32,
    /// If true, this stage is marked as a boss phase in design data.
    #[serde(default)]
    pub boss_event: bool,
}

fn default_spawn_per_second() -> f32 {
    1.0
}

impl StageConfig {
    /// Resolve effective enemies-per-second while supporting old config files.
    pub fn enemies_per_second(&self) -> f32 {
        if self.spawn_per_second > 0.0 {
            self.spawn_per_second
        } else if self.spawn_rate > 0.0 {
            1.0 / self.spawn_rate
        } else {
            1.0
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BossConfig {
    /// How many in-game minutes between boss spawns.
    pub spawn_interval_minutes: u32,
    /// Multiplier applied to the base enemy health (100 HP).
    pub health_multiplier: f32,
    /// Speed multiplier relative to base speed 40 u/s.
    pub speed_multiplier: f32,
    /// Multiplier applied to base damage (10).
    pub damage_multiplier: f32,
    /// XP dropped on death.
    pub xp_drop: i32,
}

fn load_config() -> GameConfig {
    let config_str =
        fs::read_to_string("assets/config/game_config.ron").expect("Config dosyası okunamadı");
    ron::from_str(&config_str).expect("Config dosyası parse edilemedi")
}

#[derive(Resource)]
pub struct Config(pub GameConfig);

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let config = load_config();
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: config.window.title.clone(),
                        resolution: WindowResolution::new(
                            config.window.width,
                            config.window.height,
                        ),
                        fullsize_content_view: config.window.fullscreen,
                        resizable: config.window.resizable,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(Config(config));
    }
}
