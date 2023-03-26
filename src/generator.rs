use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use fast_surface_nets::{ndshape::ConstShape, surface_nets, SurfaceNetsBuffer};
use futures_lite::future;
use rand::Rng;

use crate::{
    chunk::{ChunkData, ChunkKey},
    chunk_map::{ChunkCommandQueue, ChunkMap, DirtyChunks, LoadedChunks},
    constants::*,
    sdf_primitives::{infinite_repetition, sphere},
};

pub struct GeneratorPlugin;

impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ChunkKey>()
            .init_resource::<ChunkMap>()
            .init_resource::<ChunkCommandQueue>()
            .init_resource::<LoadedChunks>()
            .init_resource::<DirtyChunks>()
            .add_startup_system(request_chunks)
            .add_system(handle_chunk_creation_commands)
            .add_systems(
                (queue_chunk_generation_tasks, handle_chunk_generation_tasks)
                    .chain()
                    .in_set(ChunkSet::Generation),
            )
            .add_systems(
                (queue_chunk_meshing_tasks, handle_chunk_meshing_tasks)
                    .chain()
                    .in_set(ChunkSet::Meshing),
            )
            .add_system(clear_dirty_chunks.in_set(ChunkSet::Cleanup))
            .configure_set(
                ChunkSet::Cleanup
                    .after(ChunkSet::Generation)
                    .after(ChunkSet::Meshing),
            );
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum ChunkSet {
    Generation,
    Meshing,
    Cleanup,
}

fn request_chunks(
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    loaded_chunks: Res<LoadedChunks>,
) {
    info!(
        "Chunk size: {}x{}x{}",
        UnpaddedChunkShape::ARRAY[0],
        UnpaddedChunkShape::ARRAY[1],
        UnpaddedChunkShape::ARRAY[2],
    );

    let chunks_extent = Extent3i::from_min_and_lub(IVec3::splat(-10), IVec3::splat(10));
    // let chunks_extent = Extent3i::from_min_and_lub(IVec3::new(-20, -5, -20), IVec3::new(0, 0, 0));

    let chunk_count = chunks_extent.num_points();

    chunk_command_queue.create.reserve(chunk_count as usize);
    chunks_extent.iter3().map(ChunkKey::from).for_each(|key| {
        if loaded_chunks.get_entity(key).is_none() {
            chunk_command_queue.create.push(key);
        }
    });

    // TODO: replace with camera position
    chunk_command_queue.sort(ChunkKey(IVec3::ZERO));

    let voxel_count = chunk_count * (UNPADDED_CHUNK_SIZE as u64);

    info!(
        "Requested {chunk_count} chunk creation ({voxel_count} voxels) in the chunk command queue"
    );
}

fn handle_chunk_creation_commands(
    mut commands: Commands,
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    mut loaded_chunks: ResMut<LoadedChunks>,
) {
    chunk_command_queue.create.drain(..).for_each(|key| {
        let entity = commands.spawn((Name::new("Chunk"), key)).id();
        loaded_chunks.insert(key, entity);
    });
}

fn map_sdf(p: IVec3) -> Sd8 {
    let p = p.as_vec3();

    infinite_repetition(p, Vec3::splat(80.0), |q| sphere(q, 32.0)).into()
    // infinite_repetition(p, Vec3::splat(256.0), |q| sphere(q, 128.0)).into()
    // sphere(p, 640.0).into()
}

fn unpadded_chunk_extent(key: ChunkKey) -> Extent3i {
    Extent3i::from_min_and_shape(key.0 * UNPADDED_CHUNK_SHAPE, UNPADDED_CHUNK_SHAPE)
}

#[derive(Component)]
pub struct ChunkGenerationTask(Task<ChunkData>);

fn queue_chunk_generation_tasks(
    mut commands: Commands,
    added_chunks: Query<(Entity, &ChunkKey), Added<ChunkKey>>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    added_chunks.iter().for_each(|(entity, key)| {
        let unpadded_chunk_extent = unpadded_chunk_extent(*key);

        let task = task_pool.spawn(async move {
            let mut chunk_data = ChunkData::empty();

            unpadded_chunk_extent.iter3().for_each(|p| {
                let p_in_chunk = p - unpadded_chunk_extent.minimum;

                let v = &mut chunk_data.sdf
                    [UnpaddedChunkShape::linearize(p_in_chunk.as_uvec3().to_array()) as usize];

                *v = map_sdf(p);
            });

            chunk_data
        });

        commands.entity(entity).insert(ChunkGenerationTask(task));
    });
}

fn handle_chunk_generation_tasks(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    mut query: Query<(Entity, &ChunkKey, &mut ChunkGenerationTask)>,
) {
    query.for_each_mut(|(entity, key, mut task)| {
        if let Some(chunk_data) = future::block_on(future::poll_once(&mut task.0)) {
            chunk_map.insert(*key, chunk_data);
            dirty_chunks.insert(*key);
            commands.entity(entity).remove::<ChunkGenerationTask>();
        }
    });
}

#[derive(Component)]
pub struct ChunkMeshingTask(Task<Option<Mesh>>);

fn queue_chunk_meshing_tasks(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    dirty_chunks: Res<DirtyChunks>,
    loaded_chunks: Res<LoadedChunks>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    dirty_chunks
        .iter()
        .filter_map(|key| loaded_chunks.get_entity(*key).map(|entity| (key, entity)))
        .filter_map(|(key, entity)| {
            chunk_map
                .get(*key)
                .map(|chunk_data| (chunk_data.clone(), entity))
        })
        .for_each(|(chunk_data, entity)| {
            let task = task_pool.spawn(async move {
                let mut buffer = SurfaceNetsBuffer::default();

                surface_nets(
                    &chunk_data.sdf,
                    &UnpaddedChunkShape {},
                    [0; 3],
                    [UNPADDED_CHUNK_SIDE - 1; 3],
                    &mut buffer,
                );

                if buffer.positions.is_empty() {
                    return None;
                }

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    VertexAttributeValues::Float32x3(buffer.positions.clone()),
                );
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    VertexAttributeValues::Float32x3(buffer.normals.clone()),
                );
                mesh.set_indices(Some(Indices::U32(buffer.indices.clone())));

                // mesh.duplicate_vertices();
                // mesh.compute_flat_normals();

                Some(mesh)
            });

            commands.entity(entity).insert(ChunkMeshingTask(task));
        });
}

fn handle_chunk_meshing_tasks(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(Entity, &ChunkKey, &mut ChunkMeshingTask)>,
) {
    query.for_each_mut(|(entity, chunk_key, mut task)| {
        if let Some(mesh) = future::block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).remove::<ChunkMeshingTask>();

            if let Some(mesh) = mesh {
                let mesh = meshes.add(mesh);
                let material = {
                    let mut rng = rand::thread_rng();
                    let mut m = StandardMaterial::from(Color::rgb(
                        rng.gen_range(0.0..=1.0),
                        rng.gen_range(0.0..=1.0),
                        rng.gen_range(0.0..=1.0),
                    ));
                    m.perceptual_roughness = 0.6;
                    m.metallic = 0.2;
                    materials.add(m)
                };

                let chunk_min = chunk_key.0 * UNPADDED_CHUNK_SHAPE;
                let transform = Transform::from_translation(chunk_min.as_vec3());

                commands.entity(entity).insert(PbrBundle {
                    mesh,
                    material,
                    transform,
                    ..Default::default()
                });
            }
        }
    });
}

fn clear_dirty_chunks(mut dirty_chunks: ResMut<DirtyChunks>) {
    dirty_chunks.clear();
}
