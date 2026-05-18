use crate::plugins::common::GameEntity;
use crate::plugins::enemy::GameStageManager;
use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::network::{ClientEntityMap, GhostEntity, NetIdCounter, NetworkRole, PendingAudioEvents, PendingEntitySnapshots, PendingWeaponFxEvents, PendingWeaponStateEvents, VisualType, S2C};
use crate::plugins::player::{Player, flush_stat_snapshot};
use crate::plugins::score::GameScore;
use crate::plugins::texture_handling::{TextureAssets, TextureType};
use crate::plugins::timers::{EnemySpawnTimer, GameTimer, MoveTimer, PlayerHealthReduceTimer};
use crate::plugins::weapons::{
    attach_trail_effect, raygun_spark_config, spawn_explosion_effect,
    spawn_muzzle_flash, spawn_weapons_for_player, WeaponType,
};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, NoFrustumCulling};
use bevy::image::TextureAtlas;
use bevy::prelude::*;
use crate::plugins::boss::BossSpawnTracker;
use crate::plugins::config::Config;
use std::collections::HashSet;
use crate::plugins::audio::{AudioType, GameAudio};
use crate::plugins::particle_effects::{ParticleEmitter, SpawnMode};

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
            .add_systems(
                OnEnter(GameState::Loading),
                (cleanup_game, clear_client_map, reset_session_resources).chain(),
            )
            .add_systems(
                OnEnter(GameState::GameOver),
                (cleanup_game, clear_client_map, show_game_over_screen).chain(),
            )
            .add_systems(Update, restart_on_key.run_if(in_state(GameState::GameOver)))
            // Loading state
            .add_systems(
                Update,
                prepare_atlases_and_spawn.run_if(in_state(GameState::Loading)),
            )
            // Host: broadcast world state every playing frame.
            // Client: apply incoming entity snapshots to the ghost world.
            .add_systems(
                Update,
                (handle_client_weapon_fx, handle_client_weapon_state, flush_stat_snapshot, client_entity_sync, play_pending_audio_events).run_if(in_state(GameState::Playing)),
            );
    }
}

pub fn handle_client_weapon_fx(
    mut commands: Commands,
    role: Res<NetworkRole>,
    mut fx_events: ResMut<PendingWeaponFxEvents>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<ColorMaterial>>,
    texture_assets: Res<TextureAssets>,
){
    if *role != NetworkRole::Client {return;}

    for event in fx_events.0.drain(..) {
        if let S2C::WeaponFxSpawned { visual_type, transform, .. } = event {
            let _t = transform.to_transform();
            match visual_type {
                VisualType::LaserProjectile => {}
                VisualType::RocketProjectile => {}
                VisualType::SwordWeapon => {

                }
                VisualType::RayGunRay => {}
                VisualType::FlameAura => {}
                VisualType::MuzzleFlash {direction} => {spawn_muzzle_flash(&mut commands, Vec3::from(transform.translation), direction, &texture_assets);}
                VisualType::Explosion {radius} => {spawn_explosion_effect(&mut commands, Vec3::from(transform.translation),radius ,&texture_assets)}
                VisualType::Trail => {}
                _ => {}
            }
        }
    }
}

pub fn handle_client_weapon_state(
    mut commands: Commands,
    role: Res<NetworkRole>,
    mut state_events: ResMut<PendingWeaponStateEvents>,
    player_query: Query<(Entity, &Player)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
){
    if *role != NetworkRole::Client { return; }

    for event in state_events.0.drain(..) {
        // Hangi oyuncu için silah spawn edilecek bul
        if let S2C::WeaponStateChanged { net_id: _,visual_type, owner_net_id} = event {
            let target_player_entity = player_query.iter().find_map(|(e, p)| {
                if p.player_index as u32  == owner_net_id{ Some(e) } else { None }
            });

            if let Some(player_entity) = target_player_entity {
                commands.entity(player_entity).with_children(|parent| {
                    match visual_type {
                        VisualType::FlameAura => {
                            parent.spawn((
                                // GhostsEntity koymana bile gerek yok
                                Mesh2d(meshes.add(Annulus::new(0.8, 1.0))),
                                MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(0.89, 0.35, 0.13, 0.75)))),
                                Transform::default(), // Oyuncuya göre ortala
                                GlobalTransform::default(),
                                NoAutoAabb,
                            ));
                        }
                        _ => {}
                    }
                });
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct Atlases {
    pub body: Option<Handle<TextureAtlasLayout>>,
    pub shield: Option<Handle<TextureAtlasLayout>>,
    pub ready: bool,
}

fn minimal_setup(mut commands: Commands) {
    commands.spawn((Camera2d, Camera { ..default() },
        Msaa::Sample8,));
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
    config: Res<Config>,
    role: Res<NetworkRole>,
    textures_assets: Res<TextureAssets>,
) {
    if atlases.ready {
        return;
    }

    let all_loaded = textures.textures.values().all(|handle|{
        asset_server.load_state(handle).is_loaded()
    });
    if !all_loaded {
        return;
    }
    let image = match images.get(&textures.textures.get(&TextureType::Body).unwrap().clone()) {
        Some(img) => img,
        None => return,
    };

    let frame_w = (image.texture_descriptor.size.width as f32 / 9.0).round() as u32;
    let frame_h = (image.texture_descriptor.size.height as f32 / 4.0).round() as u32;
    let layout = TextureAtlasLayout::from_grid(UVec2::new(frame_w, frame_h), 9, 4, None, None);

    let body_atlas = texture_atlases.add(layout.clone());
    let shield_atlas = texture_atlases.add(layout.clone());

    atlases.body = Some(body_atlas.clone());
    atlases.shield = Some(shield_atlas);

    atlases.ready = true;

    let player_config = &config.0.player;

    // Spawn Player 1 (WASD / Host) — always spawned on all machines so both
    // screens show both characters.
    let p1 = Player {
        health: player_config.health,
        max_health: player_config.max_health,
        movement: player_config.speed,
        starting_weapon: player_config.starting_weapon.clone(),
        player_index: 0,
        ..default()
    };
    let p1_entity = commands
        .spawn((
            GameEntity,
            Sprite::from_atlas_image(
                textures.textures.get(&TextureType::Body).unwrap().clone(),
                TextureAtlas {
                    layout: body_atlas.clone(),
                    index: 0,
                },
            ),
            Transform::from_xyz(-50.0, 0.0, 0.0),
            p1,
            Aabb {
                center: Vec3::ZERO.into(),
                half_extents: Vec3::new(20., 20., 0.0).into(),
            },
            NoAutoAabb,
            NoFrustumCulling,
        ))
        .id();




    // Spawn Player 2 (Arrow keys / Client) — always spawned on all machines.
    let p2 = Player {
        health: player_config.health,
        max_health: player_config.max_health,
        movement: player_config.speed,
        starting_weapon: player_config.starting_weapon.clone(),
        player_index: 1,
        ..default()
    };
    let p2_entity = commands
        .spawn((
            GameEntity,
            Sprite::from_atlas_image(
                textures.textures.get(&TextureType::Body).unwrap().clone(),
                TextureAtlas {
                    layout: body_atlas,
                    index: 9, // slightly different sprite row to visually distinguish
                },
            ),
            Transform::from_xyz(50.0, 0.0, 0.0),
            p2,
            Aabb {
                center: Vec3::new(50., 0., 0.).into(),
                half_extents: Vec3::new(20., 20., 0.0).into(),
            },
            NoAutoAabb,
            NoFrustumCulling,
        ))
        .id();

    // Client: spawn P2's weapons; Host/Solo: also spawn P2 weapons.
    if *role == NetworkRole::Solo {
        spawn_weapons_for_player(
            &mut commands,
            p1_entity,
            Vec3::new(-50.0, 0.0, 0.0),
            &mut meshes,
            &mut materials,
            player_config.starting_weapon.as_str(),
            &textures_assets,
        );
        commands.entity(p2_entity).despawn();
    } else if *role != NetworkRole::Client {
        spawn_weapons_for_player(
            &mut commands,
            p2_entity,
            Vec3::new(50.0, 0.0, 0.0),
            &mut meshes,
            &mut materials,
            player_config.starting_weapon.as_str(),
            &textures_assets,
        );
    // Host / Solo: spawn weapons for P1.  Client has no authoritative weapon state.
        spawn_weapons_for_player(
            &mut commands,
            p1_entity,
            Vec3::new(-50.0, 0.0, 0.0),
            &mut meshes,
            &mut materials,
            player_config.starting_weapon.as_str(),
            &textures_assets,
        );
    }
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

/// Client tarafında ghost varlık haritasını sıfırla (restart / loading geçişinde).
fn clear_client_map(mut client_map: ResMut<ClientEntityMap>) {
    client_map.0.clear();
}

/// Yeni oyun oturumu için anlık kaynakları sıfırla.
///
/// `NetIdCounter` sıfırlanır ki host yeni entity ID'leri 1'den başlatsın.
/// `GameTimer` sıfırlanır ki istemci saatle uyumlu başlasın.
/// `PendingEntitySnapshots` temizlenir ki eski snapshot'lar yeni oyuna sızmasın.
fn reset_session_resources(
    mut net_id_counter: ResMut<NetIdCounter>,
    mut game_timer: ResMut<GameTimer>,
    mut pending_entity_snaps: ResMut<PendingEntitySnapshots>,
) {
    net_id_counter.0 = 0;
    game_timer.elapsed_secs = 0.0;
    pending_entity_snaps.0.clear();
}

/// GameOver ekranını göster
fn show_game_over_screen(mut commands: Commands, locale: Res<Locale>) {
    commands.spawn((
        GameEntity,
        Text::new(locale.t("game_over")),
        TextFont {
            font_size: 50.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.3, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(300.0),
            left: Val::Px(300.0),
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
    mut game_timer: ResMut<GameTimer>,
    mut stage_manager: ResMut<GameStageManager>,
    mut boss_tracker: ResMut<BossSpawnTracker>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        *atlases = Atlases::default();
        *spawn_timer = EnemySpawnTimer::default();
        *move_timer = MoveTimer::default();
        *reduce_timer = PlayerHealthReduceTimer::default();
        *game_timer = GameTimer::default();
        *stage_manager = GameStageManager::default();
        *boss_tracker = BossSpawnTracker::default();
        next_state.set(GameState::Loading);
    }
}

// ─────────────────────── Adım 3: Unified Client Sync System ──────────────────

/// Client: apply the latest entity-snapshot list from the host to the ghost world.
///
/// Algorithm:
/// 1. Despawn ghost entities whose `net_id` is no longer in the snapshot
///    (flicker-free — entity is only removed when the host stops sending it).
/// 2. For new `net_id`s: spawn a visuals-only ghost entity (Sprite or Mesh2d,
///    Transform, NoAutoAabb) — **never** add physics, damage, or AI components.
/// 3. For existing `net_id`s: update the Transform only.
pub fn client_entity_sync(
    mut commands: Commands,
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingEntitySnapshots>,
    mut client_map: ResMut<ClientEntityMap>,
    mut ghost_transforms: Query<(&mut Transform, Option<&mut Sprite>), With<GhostEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    textures: Res<TextureAssets>,
    atlases: Res<Atlases>,
    texture_assets: Res<TextureAssets>,
    move_timer: Res<MoveTimer>
) {
    if *role != NetworkRole::Client {
        return;
    }
    if pending.0.is_empty() {
        return;
    }

    let snapshots = std::mem::take(&mut pending.0);

    // ── Step 1: collect alive IDs ─────────────────────────────────────────
    let alive_ids: HashSet<u32> = snapshots.iter().map(|s| s.net_id).collect();

    // ── Step 2: despawn ghosts not present in this frame's snapshot ───────
    let dead: Vec<u32> = client_map
        .0
        .keys()
        .filter(|&&id| !alive_ids.contains(&id))
        .copied()
        .collect();
    for id in dead {
        if let Some(entity) = client_map.0.remove(&id) {
            commands.entity(entity).try_despawn();
        }
    }

    // ── Step 3: update existing / spawn new ghosts ────────────────────────
    for snap in snapshots {
        let new_transform = snap.transform.to_transform();
        if let Some(&entity) = client_map.0.get(&snap.net_id) {
            if let Ok((mut t, opt_sprite)) = ghost_transforms.get_mut(entity) {
                let diff = new_transform.translation - t.translation;
                if diff.length_squared() > 1e-6 && move_timer.timer.just_finished() {
                    if let Some(mut sprite) = opt_sprite {
                        if let Some(ref mut atlas ) = sprite.texture_atlas {
                            let direction = diff.normalize();
                            let i = (atlas.index + 1) % 9;
                            atlas.index = if direction.x.abs() > direction.y.abs() {
                                if direction.x > 0.0 { 27 + i } else { 9 + i }
                            } else {
                                if direction.y > 0.0 { 0 + i } else { 18 + i }
                            };
                        }
                    }
                }
                *t = new_transform;
            }
        }
         else {
            // First time seeing this net_id — spawn a visuals-only ghost.
            let entity = spawn_ghost(
                &mut commands,
                snap.net_id,
                snap.visual_type,
                new_transform,
                &mut meshes,
                &mut materials,
                &textures,
                &atlases,
                &texture_assets,
            );
            client_map.0.insert(snap.net_id, entity);
        }
    }
}

/// Spawn a ghost entity with **only** visual components.
///
/// Enemies receive a proper sprite sheet; projectiles and collectibles get
/// a simple colored circle so they are visible without any game-logic coupling.
fn spawn_ghost(
    commands: &mut Commands,
    net_id: u32,
    visual_type: VisualType,
    transform: Transform,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    textures: &TextureAssets,
    atlases: &Atlases,
    texture_assets: &TextureAssets,
) -> Entity {
    match visual_type {
        // ── Enemies: reuse the existing sprite atlas ──────────────────────
        VisualType::Zombie
        | VisualType::Knight
        | VisualType::Vampire
        | VisualType::Robot => {
            let texture_type = match visual_type {
                VisualType::Zombie => TextureType::Zombie,
                VisualType::Knight => TextureType::Knight,
                VisualType::Vampire => TextureType::Vampire,
                _ => TextureType::Robot,
            };
            if let (Some(tex), Some(layout)) = (
                textures.textures.get(&texture_type).cloned(),
                atlases.body.clone(),
            ) {
                return commands
                    .spawn((
                        GameEntity,
                        GhostEntity(net_id),
                        Sprite::from_atlas_image(
                            tex,
                            TextureAtlas {
                                layout,
                                index: 15,
                            },
                        ),
                        transform,
                        GlobalTransform::default(),
                        Visibility::default(),
                        NoAutoAabb,
                    ))
                    .id();
            }
            // Fallback if atlas not ready yet
            spawn_ghost_circle(commands, net_id, transform, Color::srgb(0.6, 0.0, 0.6), 20.0, meshes, materials)
        }
        // ── Collectibles ──────────────────────────────────────────────────
        VisualType::XpGem => {
            spawn_ghost_image(commands, net_id, transform, Sprite::from_image(textures.textures.get(&TextureType::XPGem).unwrap().clone()))
        }
        VisualType::Magnet => {
            spawn_ghost_image(commands, net_id, transform, Sprite::from_image(textures.textures.get(&TextureType::Magnet).unwrap().clone()))
        }VisualType::HealthPack => {
            spawn_ghost_image(commands, net_id, transform, Sprite::from_image(textures.textures.get(&TextureType::HealthPack).unwrap().clone()))
        }VisualType::AtomBomb => {
            spawn_ghost_image(commands, net_id, transform, Sprite::from_image(textures.textures.get(&TextureType::AtomBomb).unwrap().clone()))
        }
        // ── Weapon projectiles / effects ──────────────────────────────────
        VisualType::LaserProjectile => {
            let entity = spawn_ghost_image(commands, net_id, transform, Sprite::from_image(textures.textures.get(&TextureType::Laser).unwrap().clone()));
            attach_trail_effect(commands, entity, WeaponType::Laser, texture_assets);
            entity
        }
        VisualType::RocketProjectile => {
            let entity = spawn_ghost_image(commands, net_id, transform, Sprite::from_image(textures.textures.get(&TextureType::Rocket).unwrap().clone()));
            attach_trail_effect(commands, entity, WeaponType::Rocket, texture_assets);
            entity
        }
        VisualType::SwordWeapon => {
            let entity = if let Some(tex) = textures.textures.get(&TextureType::Sword) {
                spawn_ghost_image(commands, net_id, transform, Sprite::from_image(tex.clone()))
            } else {
                spawn_ghost_circle(commands, net_id, transform, Color::srgb(1.0, 1.0, 0.0), 30.0, meshes, materials)
            };
            attach_trail_effect(commands, entity, WeaponType::Sword, texture_assets);
            entity
        }
        VisualType::FlameAura => {
            spawn_ghost_circle(commands, net_id, transform, Color::srgba(1.0, 0.4, 0.0, 0.5), 100.0, meshes, materials)
        }
        VisualType::RayGunRay => {
            let entity = commands.spawn((
                    GameEntity,
                GhostEntity(net_id),
                Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.0, 1.0, 1.0)))),
                transform,
                GlobalTransform::default(),
                Visibility::default(),
                NoAutoAabb,
                ParticleEmitter {
                    enabled: true,
                    spawn_timer: Timer::from_seconds(0.04, TimerMode::Repeating),
                    particles_per_spawn: 8,
                    config: raygun_spark_config(texture_assets),
                    offset: Vec3::ZERO,
                    // Host buradaki transform.scale'i uzattıkça Particle Emitter çizgisi de çalışacaktır!
                    // Not olarak: Bu "Linear" modu 1x1 karenin boyutuna duyarlı hale getirmek için
                    // ileride ya Box kullanabilirsin ya da scale bilgisini bir sync sistemiyle Linear'ın ucuna verebilirsin.
                    // Şimdilik Box modu en temizidir, ışının sündüğü alan (scale boyutu) kadar parçacık saçar.
                    spawn_mode: SpawnMode::Box {
                        size: Vec2::new(1.0, 1.0),
                    },
                    lifetime: None,
                },
                )).id();
            entity
        }
        _ => {Entity::PLACEHOLDER}

    }
}

/// Spawn a plain colored circle ghost — used for projectiles and collectibles.
fn spawn_ghost_circle(
    commands: &mut Commands,
    net_id: u32,
    transform: Transform,
    color: Color,
    radius: f32,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) -> Entity {
    commands
        .spawn((
            GameEntity,
            GhostEntity(net_id),
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from(color))),
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            NoAutoAabb,
        ))
        .id()
}
fn spawn_ghost_image(
    commands: &mut Commands,
    net_id: u32,
    transform: Transform,
    sprite: Sprite,
) -> Entity {
    commands
        .spawn((
            GameEntity,
            GhostEntity(net_id),
            sprite,
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            NoAutoAabb,
        ))
        .id()
}

pub fn play_pending_audio_events(
    mut pending: ResMut<PendingAudioEvents>,
    audio: Res<GameAudio>,
    role: Res<NetworkRole>,
    mut commands: Commands,
){
    if *role != NetworkRole::Client {
        pending.0.clear();
        return;
    }
    for audio_type in pending.0.drain(..) {
        if let Some(audio_type) = AudioType::from_u8(audio_type) {
            audio.play_local(&mut commands, &audio_type);
        }
    }
}