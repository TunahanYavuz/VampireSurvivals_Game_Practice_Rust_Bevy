use bevy::prelude::*;
use crate::plugins::aabb::AABB;
use crate::plugins::common::GameEntity;
use crate::plugins::game_state::GameState;
use crate::plugins::player::Player;
use crate::plugins::score::GameScore;
use crate::plugins::texture_handling::TextureAssets;
use crate::plugins::timers::{EnemySpawnTimer, MoveTimer, PlayerHealthReduceTimer};
use crate::plugins::weapon_stats::spawn_weapons_for_player;
use crate::plugins::enemy::EnemyPowerUpTimer;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .init_resource::<TextureAssets>()
            .insert_resource(Atlases::default())
            // Startup systems
            .add_systems(Startup, minimal_setup)
            // State transitions
            .add_systems(OnEnter(GameState::Loading), cleanup_game)
            .add_systems(OnEnter(GameState::GameOver), (cleanup_game, show_game_over_screen).chain())
            .add_systems(Update, restart_on_key.run_if(in_state(GameState::GameOver)))
            // Loading state
            .add_systems(Update, prepare_atlases_and_spawn.run_if(in_state(GameState::Loading)));
    }
}

#[derive(Resource, Default)]
pub struct Atlases {
    pub body: Option<Handle<TextureAtlasLayout>>,
    pub shield: Option<Handle<TextureAtlasLayout>>,
    pub zombie: Option<Handle<TextureAtlasLayout>>,
    pub knight: Option<Handle<TextureAtlasLayout>>,
    pub ready: bool,
}

fn minimal_setup(mut commands: Commands) {
    commands.spawn((Camera2d, Camera { ..default() }));
}

fn prepare_atlases_and_spawn(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    textures: Res<TextureAssets>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut atlases: ResMut<Atlases>,
    mut next_state: ResMut<NextState<GameState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if atlases.ready {
        return;
    }

    if !asset_server.load_state(&textures.body).is_loaded()
        || !asset_server.load_state(&textures.shield).is_loaded()
        || !asset_server.load_state(&textures.zombie).is_loaded()
        || !asset_server.load_state(&textures.knight).is_loaded()
    {
        return;
    }

    let image = match images.get(&textures.body) {
        Some(img) => img,
        None => return,
    };

    let frame_w = (image.texture_descriptor.size.width as f32 / 9.0).round() as u32;
    let frame_h = (image.texture_descriptor.size.height as f32 / 4.0).round() as u32;
    let layout = TextureAtlasLayout::from_grid(UVec2::new(frame_w, frame_h), 9, 4, None, None);

    let body_atlas = texture_atlases.add(layout.clone());
    let shield_atlas = texture_atlases.add(layout.clone());
    let knight_atlas = texture_atlases.add(layout.clone());
    let zombie_atlas = texture_atlases.add(layout);

    atlases.body = Some(body_atlas.clone());
    atlases.shield = Some(shield_atlas);
    atlases.zombie = Some(zombie_atlas);
    atlases.knight = Some(knight_atlas);
    
    atlases.ready = true;

    // Player spawn - GameEntity marker ile işaretle
    let player_entity = commands.spawn((
        GameEntity,
        Sprite::from_atlas_image(
            textures.body.clone(),
            TextureAtlas {
                layout: body_atlas,
                index: 0,
            },
        ),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Player {
            health: 100,
            movement: 200.,
            ..default()
        },
        AABB {
            max_x: 20.,
            max_y: 20.,
            min_x: -20.,
            min_y: -20.,
            width: 40.,
            height: 40.,
        },
    )).id();
    spawn_weapons_for_player(&mut commands, player_entity, Vec3::ZERO, &mut meshes, &mut materials);
    next_state.set(GameState::Playing);
}

/// Oyun entity'lerini temizle
fn cleanup_game(
    mut commands: Commands,
    game_entities: Query<Entity, With<GameEntity>>,
    mut score: ResMut<GameScore>,
) {
    for entity in game_entities.iter() {
        commands.entity(entity).try_despawn();
    }
    score.score = 0;
}

/// GameOver ekranını göster
fn show_game_over_screen(mut commands: Commands) {
    commands.spawn((
        GameEntity,
        Text::new("Game Over! Press R to Restart"),
        TextFont {
            font_size: 50.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.3, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(300.0),
            left: Val::Px(400.0),
            ..default()
        },
    ));
}

/// R tuşu ile restart
fn restart_on_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut atlases: ResMut<Atlases>,
    mut spawn_timer: ResMut<EnemySpawnTimer>,
    mut move_timer: ResMut<MoveTimer>,
    mut reduce_timer: ResMut<PlayerHealthReduceTimer>,
    mut enemy_power: ResMut<EnemyPowerUpTimer>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        // Resource'ları resetle
        *atlases = Atlases::default();
        *spawn_timer = EnemySpawnTimer::default();
        *move_timer = MoveTimer::default();
        *reduce_timer = PlayerHealthReduceTimer::default();
        *enemy_power = EnemyPowerUpTimer::default();
        next_state.set(GameState::Loading);
    }
}

