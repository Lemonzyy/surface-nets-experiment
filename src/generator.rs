use std::sync::Arc;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    tasks::AsyncComputeTaskPool,
};
use crossbeam_queue::SegQueue;
use fast_surface_nets::{ndshape::ConstShape, surface_nets, SurfaceNetsBuffer};

use rand::Rng;

use crate::{
    chunk::{ChunkData, ChunkKey},
    chunk_map::{
        chunks_in_extent, copy_chunk_neighborhood, ChunkCommandQueue, ChunkEntityRelation,
        ChunkMap, DirtyChunks,
    },
    constants::*,
    sdf_primitives::{infinite_repetition, sphere},
};

pub struct GeneratorPlugin;

impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ChunkKey>()
            .init_resource::<ChunkMap>()
            .init_resource::<ChunkCommandQueue>()
            .init_resource::<ChunkEntityRelation>()
            .init_resource::<DirtyChunks>()
            .init_resource::<GenerationResults>()
            .init_resource::<MeshingResults>()
            .add_startup_system(request_chunks)
            .add_systems((
                spawn_chunk_generation_tasks
                    .run_if(|r: Res<ChunkCommandQueue>| !r.create.is_empty()),
                handle_chunk_generation_results.run_if(|r: Res<GenerationResults>| !r.is_empty()),
                spawn_chunk_meshing_tasks.run_if(|r: Res<DirtyChunks>| !r.is_empty()),
                handle_chunk_meshing_results.run_if(|r: Res<MeshingResults>| !r.is_empty()),
            ))
            // TODO: remove this benchmark code
            .add_system(
                |mut printed: Local<bool>,
                 time: Res<Time>,
                 q: Query<(), (With<ChunkKey>, With<Handle<Mesh>>)>| {
                    if !*printed && q.iter().len() == 8000 {
                        info!("took {:?}", time.elapsed());
                        *printed = true;
                    }
                },
            );
    }
}

#[derive(Resource, Deref, Default)]
pub struct GenerationResults(Arc<SegQueue<(ChunkKey, ChunkData)>>);

#[derive(Resource, Deref, Default)]
pub struct MeshingResults(Arc<SegQueue<(Entity, ChunkKey, Mesh)>>);

fn request_chunks(
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    loaded_chunks: Res<ChunkEntityRelation>,
) {
    info!(
        "Chunk size: {}x{}x{}",
        UnpaddedChunkShape::ARRAY[0],
        UnpaddedChunkShape::ARRAY[1],
        UnpaddedChunkShape::ARRAY[2],
    );

    let chunks_extent = Extent3i::from_min_and_lub(IVec3::splat(-10), IVec3::splat(10));
    // let chunks_extent = Extent3i::from_min_and_lub(IVec3::splat(-20), IVec3::splat(20));
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

    let point_count = chunk_count * (UNPADDED_CHUNK_SIZE as u64);

    info!(
        "Requested {chunk_count} chunk creation ({point_count} points) in the chunk command queue"
    );
}

fn map_sdf(p: IVec3) -> Sd8 {
    let p = p.as_vec3();

    infinite_repetition(p, Vec3::splat(80.0), |q| sphere(q, 32.0)).into()
    // infinite_repetition(p, Vec3::splat(256.0), |q| sphere(q, 128.0)).into()
    // sphere(p, 640.0).into()
}

fn spawn_chunk_generation_tasks(
    mut commands: Commands,
    mut chunk_command_queue: ResMut<ChunkCommandQueue>,
    mut chunk_entity_relation: ResMut<ChunkEntityRelation>,
    gen_results: Res<GenerationResults>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    chunk_command_queue.create.drain(..).for_each(|key| {
        let entity = commands.spawn((Name::new("Chunk"), key)).id();
        chunk_entity_relation.link(key, entity);

        let unpadded_chunk_extent = key.extent();

        let gen_results = Arc::clone(&gen_results);

        task_pool
            .spawn(async move {
                let mut chunk_data = ChunkData::empty();

                unpadded_chunk_extent.iter3().for_each(|p| {
                    let p_in_chunk = p - unpadded_chunk_extent.minimum;

                    let v = &mut chunk_data.sdf
                        [UnpaddedChunkShape::linearize(p_in_chunk.as_uvec3().to_array()) as usize];

                    *v = map_sdf(p);
                });

                gen_results.push((key, chunk_data));
            })
            .detach();
    });
}

fn handle_chunk_generation_results(
    mut chunk_map: ResMut<ChunkMap>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    gen_results: Res<GenerationResults>,
) {
    while let Some((key, chunk_data)) = gen_results.pop() {
        chunk_map.insert(key, chunk_data);
        dirty_chunks.insert(key);
    }
}

fn spawn_chunk_meshing_tasks(
    chunk_map: Res<ChunkMap>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    loaded_chunks: Res<ChunkEntityRelation>,
    meshing_results: Res<MeshingResults>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let mut processed_chunks = Vec::with_capacity(dirty_chunks.len());

    for &key in dirty_chunks.iter() {
        let mut neighbors = chunks_in_extent(&key.extent().padded(1));

        if !neighbors.all(|k| chunk_map.storage.contains_key(&k) || !loaded_chunks.contains(k)) {
            continue;
        }

        processed_chunks.extend(neighbors.filter(|&k| !loaded_chunks.contains(k)));

        let entity = loaded_chunks.get_entity(key).unwrap();
        let padded_sdf = copy_chunk_neighborhood(&chunk_map.storage, key);

        let meshing_results = Arc::clone(&meshing_results);
        task_pool
            .spawn(async move {
                let mut buffer = SurfaceNetsBuffer::default();

                surface_nets(
                    &padded_sdf,
                    &PaddedChunkShape {},
                    [0; 3],
                    [PADDED_CHUNK_SIDE - 1; 3],
                    &mut buffer,
                );

                if buffer.positions.is_empty() {
                    return;
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

                meshing_results.push((entity, key, mesh));
            })
            .detach();

        processed_chunks.push(key);
    }

    processed_chunks.into_iter().for_each(|k| {
        dirty_chunks.remove(&k);
    });
}

fn handle_chunk_meshing_results(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    meshing_results: Res<MeshingResults>,
) {
    while let Some((entity, key, mesh)) = meshing_results.pop() {
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

        let chunk_min = key.0 * UNPADDED_CHUNK_SHAPE;
        let transform = Transform::from_translation(chunk_min.as_vec3());

        commands.entity(entity).insert(PbrBundle {
            mesh,
            material,
            transform,
            ..Default::default()
        });
    }
}
