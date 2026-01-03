use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct GroundTile {
    pub chunk_x: i32,
    pub chunk_y: i32,
}

#[derive(Resource)]
pub struct GroundSystem {
    pub tile_size: f32,
    pub chunk_size: i32,
    pub render_distance: i32,
    pub loaded_chunks: HashSet<(i32, i32)>,
    pub tile_texture: Handle<Image>,
}

impl Default for GroundSystem {
    fn default() -> Self {
        Self {
            tile_size: 64.0,
            chunk_size: 16,
            render_distance: 4,
            loaded_chunks: HashSet::new(),
            tile_texture: Handle::default(),
        }
    }
}

/// Zemin sistemini başlat - texture'ı yükle
pub fn setup_ground(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let tile_texture = asset_server.load("textures/rpg/tiles/generic-rpg-tile01.png");

    commands.insert_resource(GroundSystem {
        tile_texture,
        ..default()
    });
}

/// Kamera konumuna göre chunk'ları dinamik olarak yükle/kaldır (SONSUZ HARİTA)
pub fn update_ground_chunks(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera2d>>,
    mut ground_system: ResMut<GroundSystem>,
    existing_tiles: Query<(Entity, &GroundTile), With<Ground>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation;

    // Kameranın hangi chunk'ta olduğunu hesapla
    let chunk_size_world = ground_system.chunk_size as f32 * ground_system.tile_size;
    let camera_chunk_x = (camera_pos.x / chunk_size_world).floor() as i32;
    let camera_chunk_y = (camera_pos.y / chunk_size_world).floor() as i32;

    // Yüklenmesi gereken chunk'ları belirle
    let mut needed_chunks = HashSet::new();
    for dx in -ground_system.render_distance..=ground_system.render_distance {
        for dy in -ground_system.render_distance..=ground_system.render_distance {
            needed_chunks.insert((camera_chunk_x + dx, camera_chunk_y + dy));
        }
    }

    // Artık gerekmeyen chunk'ları sil
    for (entity, tile) in existing_tiles.iter() {
        let chunk_coord = (tile.chunk_x, tile.chunk_y);
        if !needed_chunks.contains(&chunk_coord) {
            commands.entity(entity).despawn();
        }
    }

    // Yeni chunk'ları yükle
    for &(chunk_x, chunk_y) in needed_chunks.iter() {
        if ground_system.loaded_chunks.insert((chunk_x, chunk_y)) {
            spawn_chunk(&mut commands, &ground_system, chunk_x, chunk_y);
        }
    }

    // Yüklenmeyen chunk'ları loaded_chunks'tan kaldır
    ground_system.loaded_chunks.retain(|chunk| needed_chunks.contains(chunk));
}

/// Belirli bir chunk'ı spawn et (16x16 tile)
fn spawn_chunk(
    commands: &mut Commands,
    ground_system: &GroundSystem,
    chunk_x: i32,
    chunk_y: i32,
) {
    let chunk_size_world = ground_system.chunk_size as f32 * ground_system.tile_size;
    let chunk_origin_x = chunk_x as f32 * chunk_size_world;
    let chunk_origin_y = chunk_y as f32 * chunk_size_world;

    for local_x in 0..ground_system.chunk_size {
        for local_y in 0..ground_system.chunk_size {
            let world_x = chunk_origin_x + (local_x as f32 * ground_system.tile_size);
            let world_y = chunk_origin_y + (local_y as f32 * ground_system.tile_size);

            commands.spawn((
                Ground,
                GroundTile { chunk_x, chunk_y },
                Sprite {
                    image: ground_system.tile_texture.clone(),
                    custom_size: Some(Vec2::new(ground_system.tile_size, ground_system.tile_size)),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, -100.0),
            ));
        }
    }
}


