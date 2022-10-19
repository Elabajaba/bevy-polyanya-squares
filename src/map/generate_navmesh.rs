use bevy::{
    prelude::{debug, info, Commands, Component, Entity, IVec2, Query, Transform, Vec2},
    utils::{HashMap, HashSet, Instant},
};
use indexmap::IndexMap;
use polyanya::{Mesh as PAMesh, Polygon as PAPoly, Vertex as PAVertex};

use bevy_ecs_tilemap::{
    prelude::{TilemapGridSize, TilemapType},
    tiles::{TilePos, TileStorage},
};
use bevy_pathmesh::PathMesh;

use super::TileCost;

pub struct Connections {
    pub connection_indices: Vec<isize>,
}

impl Connections {
    pub fn new() -> Self {
        Connections {
            connection_indices: Vec::new(),
        }
    }
}

struct Polys {
    #[allow(unused)]
    position: IVec2,
    vertex_indices: [usize; 4],
}

impl Polys {
    pub fn new(position: IVec2, vertex_indices: [usize; 4]) -> Self {
        Polys {
            position,
            vertex_indices,
        }
    }
}

#[derive(Component)]
pub struct TempNavmesh {
    // pub vertices: IndexMap<IVec2, Connections>,
    // pub polygons: Vec<[usize; 4]>,
    pub debug_pa_navmesh: PAMesh,
    pub navmesh: PathMesh,
    pub dimensions: Vec2,
}

/// See https://github.com/vleue/polyanya/blob/main/meshes/format.txt
pub(crate) fn generate_map_namvesh_square_unoptimized(
    mut commands: Commands,
    tilemap_query: Query<(
        Entity,
        &TilemapType,
        &TilemapGridSize,
        &TileStorage,
        &Transform,
    )>,
    tile_query: Query<(&TilePos, &TileCost)>,
) {
    info!("trying to generate navmesh");
    let start_time = Instant::now();
    for (entity, map_type, grid_size, tilemap_storage, transform) in tilemap_query.iter() {
        // We have the vertices and their connected polygons, but not if they're an edge

        // num tiles * 1.3 sounds about right?
        let mut vertices: IndexMap<IVec2, Connections> =
            IndexMap::with_capacity((tilemap_storage.size.count() as f32 * 1.3) as usize);

        let mut poly_indices: HashMap<IVec2, isize> = HashMap::new();
        let mut highest = 0;

        let mut polygons: Vec<Polys> = Vec::with_capacity(tilemap_storage.size.count());

        for tile_entity in tilemap_storage.iter().flatten() {
            let (tile_pos, tile_cost) = tile_query.get(*tile_entity).unwrap();
            let world_pos =
                tile_pos.center_in_world(grid_size, map_type) + transform.translation.truncate();
            // println!("world_pos: {}", world_pos);
            let poly_idx = if tile_cost.0 < 1 {
                -1
            } else {
                let idx = poly_indices.entry(world_pos.as_ivec2()).or_insert(highest);
                highest += 1;
                *idx
            };
            let mut vertex_indices: [usize; 4] = [0; 4];

            [
                ((-grid_size.x / 2.0) as i32, (-grid_size.y / 2.0) as i32),
                ((grid_size.x / 2.0) as i32, (-grid_size.y / 2.0) as i32),
                ((grid_size.x / 2.0) as i32, (grid_size.y / 2.0) as i32),
                ((-grid_size.x / 2.0) as i32, (grid_size.y / 2.0) as i32),
            ]
            .iter()
            .enumerate()
            .for_each(|(idx, &corner_pos)| {
                let pos = IVec2::new(
                    corner_pos.0 + (world_pos.x) as i32,
                    corner_pos.1 + (world_pos.y) as i32,
                );
                let connections_entry = vertices.entry(pos);
                vertex_indices[idx] = connections_entry.index();
                let connections = connections_entry.or_insert(Connections::new());
                if poly_idx > -1 {
                    connections.connection_indices.push(poly_idx);
                }
            });
            if tile_cost.0 < 1 {
                continue;
            } else {
                polygons.push(Polys::new(world_pos.as_ivec2(), vertex_indices));
            }
        }

        let mut pa_vertices: Vec<PAVertex> = Vec::with_capacity(vertices.len());
        let mut pa_polys: Vec<PAPoly> = Vec::with_capacity(polygons.len());

        // TODO: Sort vertex neighbours, and also add -1 for empty polys
        // TODO: Do this properly, currently just adding None to ever vertex with <3 connections
        for (vertex_pos, connections) in vertices.iter_mut() {
            if connections.connection_indices.len() > 0 {
                // if all vertex connections are -1, then skip this vertex
                let mut temp = connections.connection_indices.clone();
                temp.sort_unstable();
                if temp[temp.len() - 1] == -1 {
                    // orphan vertex, do nothing
                    continue;
                }
            } else {
                // No connections
                continue;
            }
            if connections.connection_indices.len() < 4 {
                let mut temp: Vec<isize> = connections.connection_indices.clone();
                // TODO: This is probably in the wrong place.
                temp.push(-1);

                let vertex = PAVertex::new(vertex_pos.as_vec2(), temp);
                pa_vertices.push(vertex);
            } else {
                let vertex =
                    PAVertex::new(vertex_pos.as_vec2(), connections.connection_indices.clone());
                pa_vertices.push(vertex);
            }
        }

        for poly in polygons.iter() {
            let mut neighbours = HashSet::new();
            for vertex_idx in poly.vertex_indices {
                let connections = &vertices[vertex_idx];
                for con in connections.connection_indices.iter() {
                    neighbours.insert(*con);
                }
            }
            let is_one_way: bool = neighbours.len() <= 2;
            let temp_vertices = poly
                .vertex_indices
                .iter()
                .map(|v_idx| *v_idx as u32)
                .collect();
            let polygon = PAPoly::new(temp_vertices, is_one_way);
            pa_polys.push(polygon);
        }

        pa_vertices.shrink_to_fit();
        pa_polys.shrink_to_fit();
        debug!("Vertices len: {}", pa_vertices.len());
        debug!("polys len: {}", pa_polys.len());

        let mut navmesh = PAMesh::new(pa_vertices, pa_polys);
        let pre_bake = Instant::now();
        navmesh.bake();
        let post_bake = Instant::now();
        info!("time to bake navmesh: {:?}", post_bake - pre_bake);

        // TODO: Sort the polygons
        // let temp_polys: Vec<[usize; 4]> = polygons.iter().map(|poly| poly.vertex_indices).collect();
        let width = tilemap_storage.size.x as f32 * grid_size.x;
        let height = tilemap_storage.size.y as f32 * grid_size.y;

        commands.entity(entity).insert(TempNavmesh {
            // vertices,
            // polygons: temp_polys,
            debug_pa_navmesh: navmesh.clone(),
            navmesh: PathMesh::from_polyanya_mesh(navmesh),
            dimensions: Vec2::new(width, height),
        });
    }

    let end_time = Instant::now();
    info!("time to generate navmesh: {:?}", end_time - start_time);
}
