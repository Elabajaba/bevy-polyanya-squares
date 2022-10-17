mod generate_map;
mod generate_navmesh;

use bevy::prelude::{
    default, App, Assets, Color, Commands, Component, Mesh, PbrBundle, Plugin, Query, ResMut,
    StandardMaterial, State, SystemSet, Transform, Vec3,
};
use bevy_ecs_tilemap::{prelude::TilemapType, TilemapPlugin};
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};

use crate::{
    map::generate_map::generate_map,
    map::generate_navmesh::generate_map_namvesh_square_unoptimized, GameState,
};

pub use crate::map::generate_navmesh::TempNavmesh;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapDimensions {
            width: 204,
            height: 102,
        })
        .add_system_set(SystemSet::on_enter(GameState::MapGeneration).with_system(generate_map))
        .add_system_set(
            SystemSet::on_update(GameState::MapGeneration)
                .with_system(move_to_navmesh_state)
                .after(generate_map),
        )
        .add_system_set(
            SystemSet::on_enter(GameState::NavMeshGeneration)
                .with_system(generate_map_namvesh_square_unoptimized),
        )
        .add_system_set(
            SystemSet::on_update(GameState::NavMeshGeneration)
                .with_system(move_to_gameplay_state)
                .after(generate_map_namvesh_square_unoptimized),
        )
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(draw_navmesh))
        // .add_system(draw_navmesh)
        .add_plugin(TilemapPlugin)
        .add_plugin(DebugLinesPlugin::default());
    }
}

fn move_to_navmesh_state(mut state: ResMut<State<GameState>>) {
    state
        .set(GameState::NavMeshGeneration)
        .expect("Unable to transition from map gen state to navmesh gen state");
}

fn move_to_gameplay_state(mut state: ResMut<State<GameState>>) {
    state
        .set(GameState::Playing)
        .expect("Unable to transition from navmesh gen state to playing state");
}

#[derive(Component)]
pub struct TileCost(i8);

impl Default for TileCost {
    fn default() -> Self {
        TileCost(1)
    }
}

// #[derive(Resource)]
pub struct MapDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Component)]
struct MeshExists;

fn draw_navmesh(
    mut lines: ResMut<DebugLines>,
    navmesh_q: Query<(&TempNavmesh, &Transform, &TilemapType)>,
    // mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    // mesh_exists: Query<&MeshExists>,
) {
    for (navmesh, transform, _tilemap_type) in navmesh_q.iter() {
        // navmesh.debug_pa_navmesh.po
        // for vertex in navmesh.debug_pa_navmesh.vertices.iter() {
        //     let start = Vec3::new(vertex.coords.x, vertex.coords.y, 100.0);
        //     let end = start.clone() + Vec3::ONE;

        //     let duration = 0.0; // Duration of 0 will show the line for 1 frame.
        //     lines.line(start, end, duration);
        // }

        // for vertex in navmesh.vertices.keys() {
        //     let start = Vec3::new(
        //         ((vertex.x as f32) + transform.translation.x) * transform.scale.x,
        //         ((vertex.y as f32) + transform.translation.y) * transform.scale.y,
        //         100.0,
        //     );
        //     let end = start.clone() + Vec3::ONE;
        //     let duration = 0.0; // Duration of 0 will show the line for 1 frame.
        //     lines.line(start, end, duration);
        // }

        for (_i, polygon) in navmesh.debug_pa_navmesh.polygons.iter().enumerate() {
            [(0, 1), (1, 2), (2, 3), (3, 0)].iter().for_each(|(a, b)| {
                let start_idx = polygon.vertices[*a];
                let end_idx = polygon.vertices[*b];
                let temp1 = navmesh
                    .debug_pa_navmesh
                    .vertices
                    .get(start_idx as usize)
                    .expect(&format!(
                        "debug line start index is out of bounds: {:?}",
                        start_idx
                    ));
                let temp2 = navmesh
                    .debug_pa_navmesh
                    .vertices
                    .get(end_idx as usize)
                    .expect(&format!(
                        "debug line end index is out of bounds: {:?}",
                        end_idx
                    ));
                let start = Vec3::new(temp1.coords.x as f32, temp1.coords.y as f32, 100.0);
                let end = Vec3::new(temp2.coords.x as f32, temp2.coords.y as f32, 100.0);

                let duration = 0.0; // Duration of 0 will show the line for 1 frame.
                lines.line(start, end, duration);
            });
        }
    }
}
