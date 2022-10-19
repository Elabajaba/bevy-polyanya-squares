use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::{
    core::TaskPoolThreadAssignmentPolicy,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    math::Vec3Swizzles,
    prelude::*,
    sprite::MaterialMesh2dBundle,
    tasks::AsyncComputeTaskPool,
    // time::FixedTimestep,
    utils::Instant,
    window::WindowResized,
};

use bevy_pathmesh::PathmeshPlugin;
use bevy_prototype_debug_lines::DebugLines;

use crate::{loading::FontAssets, map::TempNavmesh, GameState};

const SPAWN_LIMIT: u64 = 10000;

pub struct MyNavPlugin;

impl Plugin for MyNavPlugin {
    fn build(&self, app: &mut App) {
        app // This example will be async heavy, increase the default threadpool
            .insert_resource(DefaultTaskPoolOptions {
                async_compute: TaskPoolThreadAssignmentPolicy {
                    min_threads: 1,
                    max_threads: usize::MAX,
                    percent: 1.0,
                },
                ..default()
            })
            .add_plugin(PathmeshPlugin)
            .init_resource::<Stats>()
            .init_resource::<NavigatorCount>()
            .insert_resource(TaskMode::Blocking)
            .insert_resource(DisplayMode::Line)
            .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(setup))
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    // .with_system(on_mesh_change)
                    .with_system(go_somewhere)
                    .with_system(compute_paths)
                    .with_system(poll_path_tasks)
                    .with_system(move_navigator)
                    .with_system(display_path)
                    .with_system(mode_change),
            )
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    // TODO: Limit this
                    // .with_run_criteria(FixedTimestep::step(0.1))
                    .with_system(spawn)
                    .with_system(update_ui),
            );
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TaskMode {
    Async,
    Blocking,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DisplayMode {
    Line,
    Nothing,
}

fn setup(mut commands: Commands, font_assets: Res<FontAssets>) {
    let font = font_assets.fira_sans.clone();

    commands.spawn_bundle(TextBundle {
        text: Text::from_sections([
            TextSection::new(
                "Agents: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "FPS: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0.0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "Task duration: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0.0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "Task overhead: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0.0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "space - ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "l - ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "\n",
                TextStyle {
                    font,
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
        ]),
        style: Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            },
            ..default()
        },
        ..default()
    });
}

#[allow(unused)]
fn on_mesh_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    navmesh_entity_q: Query<(Entity, &TempNavmesh)>,
    window_resized: EventReader<WindowResized>,
    mut wait_for_mesh: Local<bool>,
) {
    if !window_resized.is_empty() || *wait_for_mesh {
        for (entity, navmesh_container) in navmesh_entity_q.iter() {
            let pathmesh = &navmesh_container.navmesh;

            *wait_for_mesh = false;
            // if let Some(entity) = *current_mesh_entity {
            //     commands.entity(entity).despawn();
            // }

            commands.entity(entity).insert_bundle(MaterialMesh2dBundle {
                mesh: meshes.add(pathmesh.to_mesh()).into(),
                transform: Transform::from_translation(Vec3::ZERO).with_scale(Vec3::splat(1.0)),
                material: materials.add(ColorMaterial::from(Color::DARK_GRAY)),
                ..default()
            });

            // } else {
            //     *wait_for_mesh = true;
            // }
        }
    }
}

#[derive(Component)]
struct Navigator {
    speed: f32,
    color: Color,
}

#[derive(Component)]
struct Target {
    target: Vec2,
}

#[derive(Component)]
struct Path {
    path: Vec<Vec2>,
}

// #[derive(Resource)]
struct NavigatorCount(u64);

impl Default for NavigatorCount {
    fn default() -> Self {
        NavigatorCount(0)
    }
}

fn spawn(
    mut commands: Commands,
    mut navigator_count: ResMut<NavigatorCount>,
    transform_q: Query<&Transform, With<TempNavmesh>>,
) {
    if navigator_count.0 >= SPAWN_LIMIT {
        return;
    }

    let rng = fastrand::Rng::new();

    let in_mesh_starts = [
        Vec2::new(575.0, 410.0),
        Vec2::new(387.0, 524.0),
        Vec2::new(762.0, 692.0),
        Vec2::new(991.0, 426.0),
        Vec2::new(746.0, 241.0),
        Vec2::new(391.0, 231.0),
        Vec2::new(25.0, 433.0),
        Vec2::new(300.0, 679.0),
        Vec2::new(575.0, 410.0),
        Vec2::new(387.0, 524.0),
        Vec2::new(762.0, 692.0),
        Vec2::new(991.0, 426.0),
        Vec2::new(746.0, 241.0),
        Vec2::new(391.0, 231.0),
        Vec2::new(25.0, 433.0),
        Vec2::new(300.0, 679.0),
        Vec2::new(575.0, 410.0),
        Vec2::new(387.0, 524.0),
        Vec2::new(762.0, 692.0),
        Vec2::new(991.0, 426.0),
        Vec2::new(746.0, 241.0),
        Vec2::new(391.0, 231.0),
        Vec2::new(25.0, 433.0),
        Vec2::new(300.0, 679.0),
        Vec2::new(575.0, 410.0),
        Vec2::new(387.0, 524.0),
        Vec2::new(762.0, 692.0),
        Vec2::new(991.0, 426.0),
        Vec2::new(746.0, 241.0),
        Vec2::new(391.0, 231.0),
        Vec2::new(25.0, 433.0),
        Vec2::new(300.0, 679.0),
        Vec2::new(575.0, 410.0),
        Vec2::new(387.0, 524.0),
        Vec2::new(762.0, 692.0),
        Vec2::new(991.0, 426.0),
        Vec2::new(746.0, 241.0),
        Vec2::new(391.0, 231.0),
        Vec2::new(25.0, 433.0),
        Vec2::new(300.0, 679.0),
    ];

    in_mesh_starts.iter().for_each(|in_mesh| {
        navigator_count.0 += 1;
        let position = *in_mesh + transform_q.single().translation.truncate();
        let color = Color::hsl(rng.f32() * 360.0, 1.0, 0.5).as_rgba();
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::ONE),
                    ..default()
                },
                transform: Transform::from_translation(position.extend(1.0))
                    .with_scale(Vec3::splat(5.0)),
                ..default()
            })
            .insert(Navigator {
                speed: rng.f32() * 50.0 + 50.0,
                color,
            });
    });
}

#[derive(Default)]
struct TaskResult {
    path: Option<polyanya::Path>,
    done: bool,
    delay: f32,
    duration: f32,
}

#[derive(Component)]
struct FindingPath(Arc<RwLock<TaskResult>>);

fn compute_paths(
    mut commands: Commands,
    with_target: Query<(Entity, &Target, &Transform), Changed<Target>>,
    // meshes: Res<Assets<PathMesh>>,
    task_mode: Res<TaskMode>,
    mesh_query: Query<&TempNavmesh>,
    // mesh: Res<Meshes>,
) {
    let temp = mesh_query.single();
    let mesh = &temp.navmesh;
    // let mesh = if let Some(mesh) = meshes.get(&mesh.aurora) {
    //     mesh
    // } else {
    //     return;
    // };
    with_target.for_each(|(entity, target, transform)| {
        let in_mesh = transform.translation.truncate();

        let to = target.target;
        let mesh = mesh.clone();
        let finding = FindingPath(Arc::new(RwLock::new(TaskResult::default())));
        let writer = finding.0.clone();
        let start = Instant::now();
        let task_mode = *task_mode;
        AsyncComputeTaskPool::get()
            .spawn(async move {
                let delay = (Instant::now() - start).as_secs_f32();
                let path = if task_mode == TaskMode::Async {
                    mesh.get_path(in_mesh, to).await
                } else {
                    mesh.path(in_mesh, to)
                };
                *writer.write().unwrap() = TaskResult {
                    path,
                    done: true,
                    delay,
                    duration: (Instant::now() - start).as_secs_f32() - delay,
                };
            })
            .detach();
        commands.entity(entity).insert(finding);
    });
}

#[derive(Default)]
struct Stats {
    pathfinding_duration: VecDeque<f32>,
    task_delay: VecDeque<f32>,
}

fn poll_path_tasks(
    mut commands: Commands,
    computing: Query<(Entity, &FindingPath, &Transform)>,
    mut stats: ResMut<Stats>,
    // pathmeshes: Res<Assets<PathMesh>>,
    // meshes: Res<Meshes>,
    mesh_query: Query<&TempNavmesh>,
) {
    let temp = mesh_query.single();
    let mesh = &temp.navmesh;

    computing.for_each(|(entity, task, transform)| {
        let mut task = task.0.write().unwrap();
        if task.done {
            stats.pathfinding_duration.push_front(task.duration);
            stats.pathfinding_duration.truncate(100);
            stats.task_delay.push_front(task.delay);
            stats.pathfinding_duration.truncate(100);
            if let Some(path) = task.path.take() {
                commands
                    .entity(entity)
                    .insert(Path { path: path.path })
                    .remove::<FindingPath>();
            } else {
                // if !pathmeshes
                //     .get(&meshes.aurora)
                //     .unwrap()
                if !mesh.is_in_mesh(transform.translation.xy()) {
                    commands.entity(entity).despawn();
                }

                commands
                    .entity(entity)
                    .remove::<FindingPath>()
                    .remove::<Target>();
            }
        }
    });
}

fn move_navigator(
    mut query: Query<(Entity, &mut Transform, &mut Path, &Navigator)>,
    time: Res<Time>,
    mut commands: Commands,
    // mesh_q: Query<&TempNavmesh>,
) {
    // let temp = mesh_q.single();
    query.for_each_mut(|(entity, mut transform, mut path, navigator)| {
        let next = path.path[0];
        let toward = next - transform.translation.xy();
        // TODO: compare this in mesh dimensions, not in display dimensions
        if toward.length() < time.delta_seconds() * navigator.speed * 2.0 {
            path.path.remove(0);
            if path.path.is_empty() {
                commands.entity(entity).remove::<Path>().remove::<Target>();
            }
        }
        transform.translation +=
            (toward.normalize() * time.delta_seconds() * navigator.speed).extend(0.0);
    });
}

fn display_path(
    query: Query<(&Transform, &Path, &Navigator)>,
    mut lines: ResMut<DebugLines>,
    display_mode: Res<DisplayMode>,
) {
    if *display_mode == DisplayMode::Line {
        query.for_each(|(transform, path, navigator)| {
            (1..path.path.len()).for_each(|i| {
                lines.line_colored(
                    (path.path[i - 1]).extend(0f32),
                    (path.path[i]).extend(0f32),
                    0f32,
                    navigator.color,
                );
            });
            if let Some(next) = path.path.first() {
                lines.line_colored(
                    transform.translation,
                    (*next).extend(0f32),
                    0f32,
                    navigator.color,
                );
            }
        });
    }
}

fn go_somewhere(
    query: Query<
        Entity,
        (
            With<Navigator>,
            Without<Path>,
            Without<FindingPath>,
            Without<Target>,
        ),
    >,
    mesh_q: Query<(&TempNavmesh, &Transform)>,
    mut commands: Commands,
) {
    let (temp, transform) = mesh_q.single();
    let mesh_size = &temp.dimensions;
    let rng = fastrand::Rng::new();
    query.for_each(|navigator| {
        let target = Vec2::new(
            rng.f32() * mesh_size.x + transform.translation.x,
            rng.f32() * mesh_size.y + transform.translation.y,
        );
        commands.entity(navigator).insert(Target { target });
    });
}

fn update_ui(
    mut ui_query: Query<&mut Text>,
    agents: Query<&Navigator>,
    mut count: Local<usize>,
    stats: Res<Stats>,
    diagnostics: Res<Diagnostics>,
    task_mode: Res<TaskMode>,
    display_mode: Res<DisplayMode>,
) {
    let new_count = agents.iter().len();
    let mut text = ui_query.single_mut();
    text.sections[1].value = format!("{}\n", new_count);
    text.sections[3].value = format!(
        "{:.2}\n",
        diagnostics
            .get(FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.average())
            .unwrap_or_default()
    );

    text.sections[5].value = format!(
        "{:?}\n",
        Duration::from_secs_f32(
            stats.pathfinding_duration.iter().sum::<f32>()
                / (stats.pathfinding_duration.len().max(1) as f32)
        ),
    );
    text.sections[7].value = format!(
        "{:?}\n",
        Duration::from_secs_f32(
            stats.task_delay.iter().sum::<f32>() / (stats.task_delay.len().max(1) as f32)
        )
    );
    text.sections[9].value = format!("{:?}\n", *task_mode);
    text.sections[11].value = format!(
        "{}",
        match *display_mode {
            DisplayMode::Line => "hide lines",
            DisplayMode::Nothing => "display lines",
        }
    );
    *count = new_count;
}

fn mode_change(
    keyboard_input: Res<Input<KeyCode>>,
    mut task_mode: ResMut<TaskMode>,
    mut display_mode: ResMut<DisplayMode>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        match *task_mode {
            TaskMode::Async => *task_mode = TaskMode::Blocking,
            TaskMode::Blocking => *task_mode = TaskMode::Async,
        }
    }
    if keyboard_input.just_pressed(KeyCode::L) {
        match *display_mode {
            DisplayMode::Line => *display_mode = DisplayMode::Nothing,
            DisplayMode::Nothing => *display_mode = DisplayMode::Line,
        }
    }
}
