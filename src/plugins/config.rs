use serde::{Deserialize, Serialize};

use bevy::prelude::*;
use std::fs;
use bevy::window::WindowResolution;

#[derive(Debug, Deserialize, Serialize)]
pub struct GameConfig{
    pub window: WindowConfig,
    pub player: PlayerConfig,
    pub enemies: Vec<EnemyConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WindowConfig{
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub resizable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerConfig{
    pub speed: f32,
    pub health: u32,
    pub max_health: u32,
    pub starting_weapon: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnemyConfig{
    pub name: String,
    pub health: i32,
    pub speed: f32,
    pub damage: i32,
    pub spawn_rate: f32,
    pub xp_drop: i32,
}

fn load_config() -> GameConfig {
    let config_str = fs::read_to_string("assets/config/game_config.ron").expect("Config dosyası okunamadı");
    ron::from_str(&config_str).expect("Config dosyası parse edilemedi")
}

#[derive(Resource)]
pub struct Config(pub GameConfig);

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let config = load_config();
        app
            .add_plugins(DefaultPlugins.set(
                WindowPlugin{
                    primary_window: Some(Window{
                        title: config.window.title.clone(),
                        resolution: WindowResolution::new(config.window.width, config.window.height),
                        fullsize_content_view: config.window.fullscreen,
                        resizable: config.window.resizable,
                        ..default()
                    }),
                    ..default()
                }
            )
            .set(ImagePlugin::default_nearest())).insert_resource(Config(config));
    }
    
}