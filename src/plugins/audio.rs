use bevy::audio::{Volume};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_audio_assets);
    }
}

#[derive(Eq, Hash, PartialEq)]
pub enum AudioType {
    EnemyHit,
    CollectXp,
    RocketProjectileFire,
    RocketProjectileImpact,
    LaserProjectileFire,
    LaserProjectileImpact,
    SwordProjectileFire,
    SwordProjectileImpact,
    RaygunRayFire,
}

#[derive(Resource)]
pub struct GameAudio {
    pub audios: HashMap<AudioType, Handle<AudioSource>>,
}
#[derive(Component)]
pub struct GameAudioEntity;

pub fn load_audio_assets(asset_server: Res<AssetServer>, mut commands: Commands) {
    let mut audio_hash_map: HashMap<AudioType, Handle<AudioSource>> = HashMap::new();
    audio_hash_map.insert(AudioType::CollectXp, asset_server.load("sounds/breakout_collision.ogg"));
    audio_hash_map.insert(AudioType::EnemyHit, asset_server.load("sounds/Epic orchestra music.ogg"));
    audio_hash_map.insert(AudioType::RocketProjectileFire, asset_server.load("sounds/firing_rocket_projectile.ogg"));
    audio_hash_map.insert(AudioType::RocketProjectileImpact, asset_server.load("sounds/rocket_projectile_impact.ogg"));
    audio_hash_map.insert(AudioType::LaserProjectileFire, asset_server.load("sounds/firing_laser_projectile.ogg"));
    audio_hash_map.insert(AudioType::LaserProjectileImpact, asset_server.load("sounds/laser_projectile_impact.ogg"));
    audio_hash_map.insert(AudioType::SwordProjectileFire,  asset_server.load("sounds/throwing_sword_projectile.ogg"));
    audio_hash_map.insert(AudioType::SwordProjectileImpact, asset_server.load("sounds/throwing_sword_impact.ogg"));
    audio_hash_map.insert(AudioType::RaygunRayFire, asset_server.load("sounds/raygun_ray_impact.ogg"));

    commands.insert_resource(GameAudio {
        audios: audio_hash_map,
    });
}
impl GameAudio {
    pub fn play_sound(&self, commands: &mut Commands, audio_type: &AudioType, playback_mode: PlaybackSettings) {
        commands.spawn((
            AudioPlayer::new(self.audios.get(audio_type).unwrap().clone()),
            playback_mode.with_volume(Volume::Linear(0.2)),
            ));
    }
}