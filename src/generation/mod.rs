mod generator;
mod sdf;

use std::sync::Arc;

use bevy::{
    prelude::*,
    tasks::{TaskPool, TaskPoolBuilder},
};
use crossbeam_queue::SegQueue;
use fast_surface_nets::ndshape::ConstShape;

use crate::{
    chunk::{Chunk, ChunkKey, ChunkShape, Extent3i, CHUNK_SIZE},
    chunk_map::{ChunkCommand, ChunkCommandQueue, ChunkMap, CurrentChunks, DirtyChunks},
    generation::generator::GENERATOR,
    LEVEL_OF_DETAIL,
};

pub struct GenerationPlugin;

impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>()
            .init_resource::<ChunkCommandQueue>()
            .init_resource::<CurrentChunks>()
            .init_resource::<DirtyChunks>()
            .init_resource::<GenerationTaskPool>()
            .init_resource::<GenerationResults>()
            .add_startup_system(request_chunks)
            .add_systems((
                spawn_chunk_generation_tasks
                    .run_if(|r: Res<ChunkCommandQueue>| !r.is_create_empty()),
                handle_chunk_generation_results.run_if(|r: Res<GenerationResults>| !r.is_empty()),
            ));
    }
}

#[derive(Resource, Deref)]
struct GenerationTaskPool(TaskPool);

impl Default for GenerationTaskPool {
    fn default() -> Self {
        Self(
            TaskPoolBuilder::default()
                .num_threads(2)
                .thread_name("Generation Task Pool (2 threads)".to_string())
                .build(),
        )
    }
}

#[derive(Resource, Deref, Default)]
pub struct GenerationResults(Arc<SegQueue<(ChunkKey, Chunk)>>);

fn request_chunks(
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    current_chunks: Res<CurrentChunks>,
) {
    info!(
        "Chunk size: {}x{}x{}",
        ChunkShape::ARRAY[0],
        ChunkShape::ARRAY[1],
        ChunkShape::ARRAY[2],
    );

    let chunks_extent = Extent3i::from_min_and_lub(
        IVec3::splat((-10.0 / LEVEL_OF_DETAIL).floor() as i32),
        IVec3::splat((10.0 / LEVEL_OF_DETAIL).ceil() as i32),
    );
    // let chunks_extent = Extent3i::from_min_and_lub(IVec3::splat(-20), IVec3::splat(20));
    // let chunks_extent = Extent3i::from_min_and_lub(IVec3::new(-20, -5, -20), IVec3::new(0, 0, 0));

    let chunk_count = chunks_extent.num_points();

    chunks_extent
        .iter3()
        .map(ChunkKey::from)
        .filter(|&k| !current_chunks.contains(k))
        .for_each(|key| chunk_command_queue.push(ChunkCommand::Create(key)));

    // TODO: replace with camera position
    chunk_command_queue.sort_by_distance(ChunkKey(IVec3::ZERO));

    let point_count = chunk_count * (CHUNK_SIZE as u64);

    info!(
        "Requested {chunk_count} chunk creation ({point_count} points) in the chunk command queue"
    );
}

fn spawn_chunk_generation_tasks(
    gen_pool: Res<GenerationTaskPool>,
    mut commands: Commands,
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    mut current_chunks: ResMut<CurrentChunks>,
    gen_results: Res<GenerationResults>,
) {
    chunk_command_queue.drain_create_commands().for_each(|key| {
        let entity = commands.spawn((Name::new("Chunk"), key)).id();
        current_chunks.add(key, entity);

        let gen_results = Arc::clone(&gen_results);

        gen_pool
            .spawn(async move {
                let _span = trace_span!("chunk_generation_task").entered();
                let chunk_data = GENERATOR.compute_chunk(key);
                gen_results.push((key, chunk_data));
            })
            .detach();
    });*
}

fn handle_chunk_generation_results(
    mut chunk_map: ResMut<ChunkMap>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    gen_results: Res<GenerationResults>,
) {
    while let Some((key, chunk_data)) = gen_results.pop() {
        chunk_map.storage.insert(key, chunk_data);
        dirty_chunks.insert(key);
    }
}
