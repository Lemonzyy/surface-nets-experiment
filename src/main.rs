mod chunk;
mod chunk_map;
mod constants;
mod generator;
mod sdf_primitives;

use bevy::{
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::{
        settings::{WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use smooth_bevy_cameras::{
    controllers::fps::{FpsCameraBundle, FpsCameraController, FpsCameraPlugin},
    LookTransform, LookTransformPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(RenderPlugin {
            wgpu_settings: WgpuSettings {
                features: WgpuFeatures::POLYGON_MODE_LINE,
                ..Default::default()
            },
        }))
        .add_plugin(WireframePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(WorldInspectorPlugin::new())
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
    mut is_focused: Local<bool>,
) {
    if keys.just_pressed(KeyCode::F) {
        *is_focused = !*is_focused;
    }

    if *is_focused {
        let Ok(mut look_transform) = camera_query.get_single_mut() else {
            return;
        };

        look_transform.target = Vec3::ZERO;
    }
}
