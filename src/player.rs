use crate::actions::Actions;
// use crate::loading::TextureAssets;
use crate::many::MyNavPlugin;
// use crate::map::TempNavmesh;
use crate::GameState;
use bevy::prelude::*;

pub struct PlayerPlugin;

#[derive(Component)]
pub struct Player;

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Playing).with_system(spawn_player))
            .add_system_set(SystemSet::on_update(GameState::Playing).with_system(move_player))
            // .add_system_set(
            //     SystemSet::on_update(GameState::Playing).with_system(check_mouse_navmesh),
            // )
            .add_plugin(MyNavPlugin);
    }
}

fn spawn_player(mut commands: Commands, camera_q: Query<Entity, With<Camera>>) {
    let cam_entity = camera_q.single();
    commands.entity(cam_entity).insert(Player);
}

fn move_player(
    time: Res<Time>,
    actions: Res<Actions>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    if actions.player_movement.is_none() {
        return;
    }
    let speed = 250.;
    let movement = Vec3::new(
        actions.player_movement.unwrap().x * speed * time.delta_seconds(),
        actions.player_movement.unwrap().y * speed * time.delta_seconds(),
        0.,
    );
    for mut player_transform in &mut player_query {
        player_transform.translation += movement;
    }
}

// fn check_mouse_navmesh(
//     mesh_q: Query<&TempNavmesh>,
//     windows: Res<Windows>,
//     q_camera: Query<(&Camera, &GlobalTransform)>,
// ) {
//     let temp = mesh_q.single();
//     let navmesh = &temp.navmesh;
//     let ab = &temp.debug_pa_navmesh;
//     // let dimensions = &temp.dimensions;

//     let window = windows.get_primary().unwrap();

//     if let Some(position) = window.cursor_position() {
//         // cursor is inside the window, position given
//         let (camera, camera_transform) = q_camera.single();
//         // get the size of the window
//         let window_size = Vec2::new(window.width() as f32, window.height() as f32);

//         // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
//         let ndc = (position / window_size) * 2.0 - Vec2::ONE;

//         // matrix for undoing the projection and camera transform
//         let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

//         // use it to convert ndc to world-space coordinates
//         let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

//         // reduce it to a 2D value
//         let world_pos: Vec2 = world_pos.truncate();

//         // position.
//         if ab.point_in_mesh(world_pos) {
//             println!("normal point: {} is in mesh", world_pos);
//         }
//         if navmesh.is_in_mesh(world_pos) {
//             println!("point: {} is in mesh", world_pos);
//         } else {
//             // println!("adhksfjksdfhlafhjka point: {} is not in mesh", world_pos);
//         }
//     } else {
//         // cursor is not inside the window
//     }
// }
