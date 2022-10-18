use bevy::{
    prelude::{info, Commands, Res},
    utils::Instant,
};
use bevy_ecs_tilemap::{
    prelude::{
        get_tilemap_center_transform, TilemapId, TilemapSize, TilemapTexture, TilemapTileSize,
    },
    tiles::{TileBundle, TilePos, TileStorage, TileTexture},
    TilemapBundle,
};

use crate::loading::TextureAssets;

use super::{MapDimensions, TileCost};

pub(crate) fn generate_map(
    mut commands: Commands,
    textures: Res<TextureAssets>,
    map_dimensions: Res<MapDimensions>,
) {
    let start_time = Instant::now();

    let tilemap_size = TilemapSize {
        x: map_dimensions.width,
        y: map_dimensions.height,
    };
    let mut tile_storage = TileStorage::empty(tilemap_size);
    let tilemap_entity = commands.spawn().id();
    let rng = fastrand::Rng::with_seed(1);

    for x in 0..map_dimensions.width {
        for y in 0..map_dimensions.height {
            let tile_pos = TilePos { x, y };
            let tile_cost = rng.i8(-2..8);
            let texture = if tile_cost < 1 { 1 } else { 0 };
            let tile_entity = commands
                .spawn()
                .insert_bundle(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture: TileTexture(texture),
                    ..Default::default()
                })
                .insert(TileCost(tile_cost))
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = tile_size.into();

    commands
        .entity(tilemap_entity)
        .insert_bundle(TilemapBundle {
            grid_size,
            size: tilemap_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(textures.tiles_texture.clone()),
            tile_size,
            transform: get_tilemap_center_transform(&tilemap_size, &grid_size, 0.0),
            ..Default::default()
        });

    let end_time = Instant::now();
    info!("time to generate map: {:?}", end_time - start_time);
}
