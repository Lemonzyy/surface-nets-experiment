mod chunk;
mod chunk_map;
mod debug;
mod generation;
mod meshing;

use bevy::{
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::{
        settings::{WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
    window::{Cursor, CursorGrabMode},
};

use smooth_bevy_cameras::{
    controllers::fps::{FpsCameraBundle, FpsCameraController, FpsCameraPlugin},
    LookTransform, LookTransformPlugin,
};

/// 2.0 means half the detail
///
/// TODO: make it dynamic
const LEVEL_OF_DETAIL: f32 = 1.0;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    wgpu_settings: WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..Default::default()
                    },
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        cursor: {
                            let mut cursor = Cursor::default();
                            cursor.visible = false;
                            cursor.grab_mode = CursorGrabMode::Locked; // currently doesn't work I don't know why :/
                            cursor
                        },
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugin(WireframePlugin)
        .add_plugin(bevy_egui::EguiPlugin)
        // .add_plugin(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_plugin(LookTransformPlugin)
        .add_plugin(FpsCameraPlugin::default())
        .add_plugin(generation::GenerationPlugin)
        .add_plugin(meshing::MeshingPlugin)
        .add_plugin(debug::DebugPlugin)
        .add_startup_system(setup)
        .add_systems((camera_focus_origin, toggle_cursor_and_camera))
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    info!("Starting up!");

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::PI / 4.),
            ..default()
        },
        ..default()
    });

    commands
        .spawn(Camera3dBundle::default())
        .insert(FpsCameraBundle::new(
            FpsCameraController {
                translate_sensitivity: 150.0,
                ..default()
            },
            Vec3::splat(500.0),
            Vec3::ZERO,
            Vec3::Y,
        ));

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube::new(8.0))),
        material: materials.add(Color::RED.into()),
        transform: Transform::IDENTITY,
        ..default()
    });
}

fn camera_focus_origin(
    keys: Res<Input<KeyCode>>,
    mut cameras: Query<&mut LookTransform, With<FpsCameraController>>,
    mut is_focused: Local<bool>,
) {
    if keys.just_pressed(KeyCode::F) {
        *is_focused = !*is_focused;
    }

    if *is_focused {
        let Ok(mut look_transform) = cameras.get_single_mut() else {
            return;
        };

        look_transform.target = Vec3::ZERO;
    }
}

fn toggle_cursor_and_camera(
    keys: Res<Input<KeyCode>>,
    mut windows: Query<&mut Window>,
    mut cameras: Query<&mut FpsCameraController>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        let mut window = windows.single_mut();
        window.cursor.visible = !window.cursor.visible;
        window.cursor.grab_mode = match window.cursor.grab_mode {
            CursorGrabMode::None => CursorGrabMode::Locked,
            CursorGrabMode::Confined | CursorGrabMode::Locked => CursorGrabMode::None,
        };

        let mut camera = cameras.single_mut();
        camera.enabled = !camera.enabled;
    }
}
