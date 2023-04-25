use std::sync::Arc;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    tasks::{TaskPool, TaskPoolBuilder},
};
use crossbeam_queue::SegQueue;
use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};

use rand::Rng;

use crate::{
    chunk::{ChunkKey, PaddedChunkShape, CHUNK_SHAPE, PADDED_CHUNK_SHAPE, PADDED_CHUNK_SIDE},
    chunk_map::{chunks_in_extent, ChunkMap, CurrentChunks, DirtyChunks},
    LEVEL_OF_DETAIL,
};

pub struct MeshingPlugin;

impl Plugin for MeshingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeshingTaskPool>()
            .init_resource::<MeshingResults>()
            .add_systems((
                spawn_chunk_meshing_tasks.run_if(|r: Res<DirtyChunks>| !r.is_empty()),
                handle_chunk_meshing_results.run_if(|r: Res<MeshingResults>| !r.is_empty()),
            ));
    }
}

#[derive(Resource, Deref)]
struct MeshingTaskPool(TaskPool);

impl Default for MeshingTaskPool {
    fn default() -> Self {
        Self(
            TaskPoolBuilder::default()
                .num_threads(2)
                .thread_name("Meshing Task Pool (2 threads)".to_string())
                .build(),
        )
    }
}

#[derive(Resource, Deref, Default)]
pub struct MeshingResults(Arc<SegQueue<(Entity, ChunkKey, Mesh)>>);

fn spawn_chunk_meshing_tasks(
    meshing_pool: Res<MeshingTaskPool>,
    chunk_map: Res<ChunkMap>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    current_chunks: Res<CurrentChunks>,
    meshing_results: Res<MeshingResults>,
) {
    let mut processed_chunks = Vec::with_capacity(dirty_chunks.len());

    for &key in dirty_chunks.iter() {
        let mut neighbors = chunks_in_extent(&key.extent().with_shape(PADDED_CHUNK_SHAPE));

        if !neighbors.all(|k| chunk_map.storage.contains_key(&k) || !current_chunks.contains(k)) {
            continue;
        }

        processed_chunks.extend(neighbors.filter(|&k| !current_chunks.contains(k)));

        let entity = current_chunks.get_entity(key).unwrap();
        let padded_sdf = chunk_map.copy_chunk_neighborhood(key);

        let meshing_results = Arc::clone(&meshing_results);
        meshing_pool
            .spawn(async move {
                let _span = trace_span!("chunk_meshing_task").entered();
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
                rng.gen_range(0.0..=1.0), //0.168 ,
                rng.gen_range(0.0..=1.0), //0.133 ,
                rng.gen_range(0.0..=1.0), //0.102 ,
            ));
            m.perceptual_roughness = 0.6;
            m.metallic = 0.2;
            materials.add(m)
        };

        let chunk_min = key.0 * CHUNK_SHAPE;
        let transform = Transform::from_translation(chunk_min.as_vec3() * LEVEL_OF_DETAIL)
            .with_scale(Vec3::splat(LEVEL_OF_DETAIL));

        commands.entity(entity).insert(PbrBundle {
            mesh,
            material,
            transform,
            ..Default::default()
        });
    }
}
