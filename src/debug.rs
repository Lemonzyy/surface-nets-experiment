use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::{
    chunk::ChunkKey,
    chunk_map::{ChunkCommandQueue, ChunkMap, DirtyChunks},
    generator::{ChunkGenerationTask, ChunkMeshingTask},
};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugUiState>().add_systems((
            ui_debug,
            update_added_chunk_count,
            update_generation_tasks_count,
            update_meshing_tasks_count,
        ));
    }
}

#[derive(Resource, Default)]
struct DebugUiState {
    chunk_key: (i32, i32, i32),
    added_chunk_count: usize,
    generation_tasks_count: usize,
    meshing_tasks_count: usize,
}

fn ui_debug(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<DebugUiState>,
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    dirty_chunks: Res<DirtyChunks>,
    chunk_map: Res<ChunkMap>,
) {
    egui::Window::new("Debug").show(contexts.ctx_mut(), |ui| {
        for (k, v) in [
            ("Chunk creation command", chunk_command_queue.create.len()),
            ("Chunk deletion command", chunk_command_queue.delete.len()),
            ("Added chunk", ui_state.added_chunk_count),
            ("Dirty chunks", dirty_chunks.len()),
            ("Chunk map entries", chunk_map.len()),
            ("generation_tasks_count", ui_state.generation_tasks_count),
            ("meshing_tasks_count", ui_state.meshing_tasks_count),
        ] {
            ui.label(format!("{k}: {v}"));
        }

        ui.separator();

        ui.label("Configure chunk:");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut ui_state.chunk_key.0));
            ui.add(egui::DragValue::new(&mut ui_state.chunk_key.1));
            ui.add(egui::DragValue::new(&mut ui_state.chunk_key.2));
        });
        if ui.button("Add chunk").clicked() {
            chunk_command_queue
                .create
                .push(ChunkKey(IVec3::from(ui_state.chunk_key)));
        }
    });
}

fn update_added_chunk_count(
    mut ui_state: ResMut<DebugUiState>,
    query: Query<Entity, Added<ChunkKey>>,
) {
    ui_state.added_chunk_count = query.iter().count();
}

fn update_generation_tasks_count(
    mut ui_state: ResMut<DebugUiState>,
    query: Query<(), (With<ChunkKey>, With<ChunkGenerationTask>)>,
) {
    ui_state.generation_tasks_count = query.iter().count();
}

fn update_meshing_tasks_count(
    mut ui_state: ResMut<DebugUiState>,
    query: Query<(), (With<ChunkKey>, With<ChunkMeshingTask>)>,
) {
    ui_state.meshing_tasks_count = query.iter().count();
}
