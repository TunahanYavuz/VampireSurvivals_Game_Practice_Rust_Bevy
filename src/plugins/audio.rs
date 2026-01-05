use bevy::prelude::*;
use bevy::audio::{AudioSink, PlaybackMode};

#[derive(Resource, Clone)]
pub struct GameAudio {
    pub enemy_hit: Handle<AudioSource>,
    pub collect_xp: Handle<AudioSource>,
}

pub fn load_audio_assets(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let enemy_hit = asset_server.load("sounds/breakout_collision.ogg");
    let collect_xp = asset_server.load("sounds/Epic orchestra music.ogg");

    commands.insert_resource(GameAudio {
        enemy_hit,
        collect_xp,
    });
}