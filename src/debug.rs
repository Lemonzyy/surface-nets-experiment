use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{egui, EguiContexts};

use crate::{
    chunk::ChunkKey,
    chunk_map::{ChunkCommand, ChunkCommandQueue, ChunkMap, DirtyChunks},
    generation::GenerationResults,
    meshing::MeshingResults,
};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(FrameTimeDiagnosticsPlugin)
            .init_resource::<DebugUiState>()
            .add_system(ui_debug);
    }
}

#[derive(Resource, Default)]
struct DebugUiState {
    chunk_key: (i32, i32, i32),
}

fn ui_debug(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<DebugUiState>,
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    diagnostics: Res<Diagnostics>,
    added_chunks: Query<Entity, Added<ChunkKey>>,
    dirty_chunks: Res<DirtyChunks>,
    chunk_map: Res<ChunkMap>,
    gen_results: Res<GenerationResults>,
    meshing_results: Res<MeshingResults>,
) {
    egui::Window::new("Debug").show(contexts.ctx_mut(), |ui| {
        ui.label(format!(
            "Average FPS: {:.02}",
            diagnostics
                .get(FrameTimeDiagnosticsPlugin::FPS)
                .unwrap()
                .average()
                .unwrap_or_default()
        ));

        ui.separator();

        for (k, v) in [
            ("Chunk creation commands", chunk_command_queue.create_len()),
            ("Chunk deletion commands", chunk_command_queue.delete_len()),
            ("Added chunks", added_chunks.iter().count()),
            ("Dirty chunks", dirty_chunks.len()),
            ("Chunk map entries", chunk_map.storage.len()),
            ("Generation results", gen_results.len()),
            ("Meshing results", meshing_results.len()),
        ] {
            ui.label(format!("{k}: {v}"));
        }

        ui.separator();

        ui.label("Chunk key:");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut ui_state.chunk_key.0));
            ui.add(egui::DragValue::new(&mut ui_state.chunk_key.1));
            ui.add(egui::DragValue::new(&mut ui_state.chunk_key.2));
        });
        if ui.button("Add chunk").clicked() {
            let chunk_key = ChunkKey(IVec3::from(ui_state.chunk_key));
            chunk_command_queue.push(ChunkCommand::Create(chunk_key));
        }
    });
}
