use crate::plugins::aabb::AABB;
use crate::plugins::enemy::*;
use crate::plugins::player::*;
use crate::plugins::texture_handling::TextureAssets;
use crate::plugins::timers::*;
use crate::plugins::weapons::*;
use crate::plugins::game_state::GameState;
use crate::plugins::ground::{setup_ground, update_ground_chunks};
use bevy::asset::AssetServer;
use bevy::prelude::*;
use crate::plugins::audio::load_audio_assets;
use crate::plugins::main_menu::MainMenuPlugin;
use crate::plugins::score::{setup_score_ui, update_score_ui, GameScore};
use crate::plugins::weapon_stats::spawn_weapons_for_player;
use crate::plugins::weapon_upgrade::*;

mod plugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .insert_state(GameState::MainMenu)
        
        // Resources
        .init_resource::<GameScore>()
        .init_resource::<UpgradeChoices>()
        .init_resource::<EnemyPowerUpTimer>()
        // Events
        .add_message::<LevelUpEvent>()
        .add_message::<UpgradeSelectedEvent>()
        
        // Resources
        .init_resource::<TextureAssets>()
        .insert_resource(Atlases::default())
        .add_systems(Startup, (minimal_setup, setup_score_ui, setup_ground, load_audio_assets))
        .init_resource::<EnemySpawnTimer>()
        .init_resource::<MoveTimer>()
        .init_resource::<PlayerHealthReduceTimer>()
        .add_plugins(MainMenuPlugin)
        .add_systems(
            Update,
            (
                prepare_atlases_and_spawn.run_if(in_state(GameState::Loading)),
                (
                    collect_xp_with_magnet,
                    magnetite_xp_to_player,
                    enemy_collision_with_enemy,
                    update_score_ui,
                    update_ground_chunks,
                    follow,
                    move_player_addicted_weapons,
                    fire_laser_weapons,
                    fire_rocket_weapons,
                    move_player,
                    spawn_enemies,
                    reduce_player_health,
                    move_projectiles,
                    despawn_explosions,
                    collect_xp,

                ).run_if(in_state(GameState::Playing)),
            ),
        )
        .add_systems(Update, (show_upgrade_choices_on_level_up,
                     handle_upgrade_input, apply_weapon_upgrade).run_if(in_state(GameState::UpgradeSelection)))
        .add_systems(OnEnter(GameState::Loading), cleanup_game)
        .add_systems(OnEnter(GameState::GameOver), (cleanup_game, show_game_over_screen).chain())
        .add_systems(OnExit(GameState::UpgradeSelection), cleanup_upgrade_ui_on_choice)
        .add_systems(OnEnter(GameState::UpgradeSelection), create_table_ui)
        .add_systems(Update, restart_on_key.run_if(in_state(GameState::GameOver)))
        .run();
}

// Marker component - oyun sırasında oluşturulan tüm entity'lere eklenecek

#[derive(Resource, Default)]
struct Atlases {
    body: Option<Handle<TextureAtlasLayout>>,
    shield: Option<Handle<TextureAtlasLayout>>,
    ready: bool,
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
    let shield_atlas = texture_atlases.add(layout);

    atlases.body = Some(body_atlas.clone());
    atlases.shield = Some(shield_atlas.clone());
    atlases.ready = true;

    // Player spawn - GameEntity marker ile işaretle
    let player_entity = commands.spawn((
        GameEntity,  // ← Marker eklendi
        Sprite::from_atlas_image(
            textures.body.clone(),
            TextureAtlas {
                layout: body_atlas,
                index: 0,
            },
        ),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Player {
            health: 1,
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


fn move_player(
    mut player_query: Query<(&mut Transform, &Player, &mut AABB, &mut Sprite), With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    atlases: Res<Atlases>,
    enemy_move_timer: Res<MoveTimer>,
) {
    if !atlases.ready {
        return;
    }

    // Single yerine Query kullanıp güvenli kontrol
    let Ok((mut transform, player, mut aabb, mut sprite)) = player_query.single_mut() else {
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    if sprite.texture_atlas.is_none() {
        if let Some(layout_handle) = &atlases.body {
            sprite.texture_atlas = Some(TextureAtlas {
                layout: layout_handle.clone(),
                index: 0,
            });
        }
    }

    player.move_around(
        &mut transform,
        &mut aabb,
        &mut sprite,
        &mut camera_transform,
        &keyboard_input,
        &time,
        &enemy_move_timer,
    );
}

fn reduce_player_health(
    mut commands: Commands,
    mut player_query: Query<(&mut Player, &mut AABB, Entity), With<Player>>,
    enemy_query: Query<(&AABB, &Enemy), (With<Enemy>, Without<Player>)>,
    mut player_health_reduce_timer: ResMut<PlayerHealthReduceTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    player_health_reduce_timer.timer.tick(time.delta());
    if !player_health_reduce_timer.timer.just_finished() {
        return;
    }

    let Ok((mut player, aabb, entity)) = player_query.single_mut() else {
        return;
    };

    player.take_damage(entity, &mut commands, enemy_query, &aabb);

    if player.health == 0 {
        next_state.set(GameState::GameOver);
    }
}

// OnExit(GameState::Playing) ile tetiklenir - sadece GameEntity olanları temizle
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

// GameOver ekranını göster
fn show_game_over_screen(mut commands: Commands) {
    commands.spawn((
        GameEntity,  // Bu da oyun entity'si, tekrar restart olunca silinecek
        Text::new("Game Over! Press R to Restart"),
        TextFont {
            font_size: 50.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.3, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: px(300.0),
            left: px(400.0),
            ..default()
        },
    ));
}

// R tuşu ile restart
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
        // State'i değiştir - OnExit(Playing) tetiklenmeyecek çünkü Playing'den çıkmıyoruz
        // GameOver'dan Loading'e geçiyoruz
        next_state.set(GameState::Loading);
    }
}
