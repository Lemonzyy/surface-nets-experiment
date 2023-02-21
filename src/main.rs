mod chunk;
mod chunk_map;
mod constants;
mod generator;

use bevy::{
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::settings::{WgpuFeatures, WgpuSettings},
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use smooth_bevy_cameras::{
    controllers::fps::{FpsCameraBundle, FpsCameraController, FpsCameraPlugin},
    LookTransform, LookTransformPlugin,
};

fn main() {
    App::new()
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::POLYGON_MODE_LINE,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(WireframePlugin)
        .add_plugin(WorldInspectorPlugin)
        .add_plugin(LookTransformPlugin)
        .add_plugin(FpsCameraPlugin::default())
        .add_plugin(generator::GeneratorPlugin)
        .add_startup_system(setup)
        .add_system(camera_focus_origin)
        .run();
}

fn setup(mut commands: Commands) {
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
            Vec3::splat(750.0),
            Vec3::ZERO,
            Vec3::Y,
        ));
}

fn camera_focus_origin(
    keys: Res<Input<KeyCode>>,
    mut camera_query: Query<&mut LookTransform, With<FpsCameraController>>,
) {
    if keys.just_pressed(KeyCode::F) {
        let Ok(mut look_transform) = camera_query.get_single_mut() else {
            return;
        };

        look_transform.target = Vec3::ZERO;
    }
}
