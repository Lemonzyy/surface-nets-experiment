use std::sync::Arc;

use bevy::{
    math::Vec3A,
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use fast_surface_nets::{ndshape::ConstShape, surface_nets, SurfaceNetsBuffer};
use futures_lite::future;
use parking_lot::Mutex;
use rand::Rng;

use crate::{
    chunk::{Chunk, ChunkCoord, ChunkGenerated, ChunkMeshed},
    chunk_map::ChunkMap,
    constants::*,
};

pub struct GeneratorPlugin;

impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ChunkCoord>()
            .init_resource::<ChunkMap>()
            .init_resource::<ChunkMapDebug>()
            .register_type::<ChunkMapDebug>()
            .add_plugin(ResourceInspectorPlugin::<ChunkMapDebug>::default())
            .add_startup_system(spawn_chunks)
            .add_system(spawn_chunk_generation_tasks)
            .add_system(handle_chunk_generation_tasks)
            .add_system(spawn_chunk_meshing_tasks)
            .add_system(handle_chunk_meshing_tasks)
            .add_system(debug_generation_tasks)
            .add_system(debug_meshing_tasks)
            .add_system(debug_generated_chunks)
            .add_system(debug_meshed_chunks);
    }
}

fn spawn_chunks(mut commands: Commands) {
    let chunks_extent = Extent3i::from_min_and_lub(IVec3::splat(-10), IVec3::splat(10));
    // let chunks_extent = Extent3i::from_min_and_lub(IVec3::new(-20, -5, -20), IVec3::new(0, 0, 0));

    let chunk_entities = chunks_extent
        .iter3()
        .map(|c| spawn_chunk(c, &mut commands))
        .collect::<Vec<_>>();

    commands
        .spawn((Name::new("Chunks"), SpatialBundle::default()))
        .push_children(&chunk_entities);

    let chunk_count = chunks_extent.num_points();
    let voxel_count = chunk_count * (UNPADDED_CHUNK_SIZE as u64);
    info!(
        "Spawning {chunk_count} chunks ({}x{}x{}) for a total of {voxel_count} voxels",
        UnpaddedChunkShape::ARRAY[0],
        UnpaddedChunkShape::ARRAY[1],
        UnpaddedChunkShape::ARRAY[2],
    );
}

fn spawn_chunk(coord: IVec3, commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new(format!(
                "Chunk {{ x:{}, y:{}, z:{} }}",
                coord.x, coord.y, coord.z
            )),
            ChunkCoord(coord),
        ))
        .id()
}

fn signed_distance_function(p: IVec3) -> Sd8 {
    let pf = p.as_vec3a();
    let c = Vec3A::splat(65.0);
    // q = mod(pf+0.5*c,c)-0.5*c;
    let q = modulo(pf + 0.5 * c, c) - 0.5 * c;

    // From https://registry.khronos.org/OpenGL-Refpages/gl4/html/mod.xhtml
    fn modulo(x: Vec3A, y: Vec3A) -> Vec3A {
        x - y * (x / y).floor()
    }

    fn sphere(p: Vec3A) -> Sd8 {
        (p.length() - 32.0).into()
    }

    sphere(q)
}

#[derive(Component)]
struct ChunkGenerationTask(Task<Chunk>);

#[allow(clippy::type_complexity)]
fn spawn_chunk_generation_tasks(
    mut commands: Commands,
    query: Query<(Entity, &ChunkCoord), (Without<ChunkGenerationTask>, Without<ChunkGenerated>)>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;
        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let unpadded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, UNPADDED_CHUNK_SHAPE);

        let task = task_pool.spawn(async move {
            let mut chunk = Chunk {
                entity: Some(entity),
                ..default()
            };

            unpadded_chunk_extent.iter3().for_each(|p| {
                let p_in_chunk = p - unpadded_chunk_extent.minimum;

                let v = &mut chunk.data
                    [UnpaddedChunkShape::linearize(p_in_chunk.as_uvec3().to_array()) as usize];

                *v = signed_distance_function(p);
            });

            chunk
        });

        commands.entity(entity).insert(ChunkGenerationTask(task));
    }
}

fn handle_chunk_generation_tasks(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut query: Query<(Entity, &ChunkCoord, &mut ChunkGenerationTask)>,
) {
    for (entity, chunk_coord, mut task) in &mut query {
        if let Some(chunk) = future::block_on(future::poll_once(&mut task.0)) {
            chunk_map.insert_chunk(chunk_coord.0, chunk);
            commands
                .entity(entity)
                .insert(ChunkGenerated)
                .remove::<ChunkGenerationTask>();
        }
    }
}

#[derive(Component)]
struct ChunkMeshingTask(Task<Option<Mesh>>);

#[allow(clippy::type_complexity)]
fn spawn_chunk_meshing_tasks(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    query: Query<
        (Entity, &ChunkCoord),
        (
            With<ChunkGenerated>,
            Without<ChunkMeshingTask>,
            Without<ChunkMeshed>,
        ),
    >,
) {
    let task_pool = AsyncComputeTaskPool::get();

    'query_loop: for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;
        let meshing_chunk_coords = MESHING_CHUNKS_OFFSET.map(|offset| chunk_coord + offset);

        // FIXME: store pending chunks to optimize this --> some chunks can't be meshed at the boundaries of the chunk map
        for coord in &meshing_chunk_coords {
            if chunk_map.get_chunk(coord).is_none() {
                continue 'query_loop;
            }
        }

        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let padded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, PADDED_CHUNK_SHAPE);

        let mut samples = [DEFAULT_SDF_VALUE; PADDED_CHUNK_SIZE];

        let meshing_chunk_intersection_extents = meshing_chunk_coords
            .map(|chunk| chunk * UNPADDED_CHUNK_SHAPE)
            .map(|chunk_min| Extent3i::from_min_and_shape(chunk_min, UNPADDED_CHUNK_SHAPE))
            .map(|adj_chunk_ext| padded_chunk_extent.intersection(&adj_chunk_ext));

        MESHING_CHUNKS_OFFSET
            .into_iter()
            .zip(meshing_chunk_coords.into_iter())
            .zip(meshing_chunk_intersection_extents)
            .for_each(|((offset, chunk), intersection_extent)| {
                if let Some(chunk) = chunk_map.get_chunk(&chunk) {
                    ndcopy::copy3(
                        intersection_extent.shape.as_uvec3().to_array(),
                        &chunk.data,
                        &UnpaddedChunkShape {},
                        [0; 3],
                        &mut samples,
                        &PaddedChunkShape {},
                        (offset * UNPADDED_CHUNK_SHAPE).as_uvec3().to_array(),
                    );
                }
            });

        let task = task_pool.spawn(async move {
            let mut buffer = SurfaceNetsBuffer::default();

            surface_nets(
                &samples,
                &PaddedChunkShape {},
                [0; 3],
                [UNPADDED_CHUNK_SIDE + CHUNK_PADDING; 3],
                &mut buffer,
            );

            if !buffer.positions.is_empty() {
                let num_vertices = buffer.positions.len();

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    VertexAttributeValues::Float32x3(buffer.positions.clone()),
                );
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    VertexAttributeValues::Float32x3(buffer.normals.clone()),
                );
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
                );
                mesh.set_indices(Some(Indices::U32(buffer.indices.clone())));

                // mesh.duplicate_vertices();
                // mesh.compute_flat_normals();

                Some(mesh)
            } else {
                None
            }
        });

        commands.entity(entity).insert(ChunkMeshingTask(task));
    }
}

fn handle_chunk_meshing_tasks(
    commands: Commands,
    materials: ResMut<Assets<StandardMaterial>>,
    meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(Entity, &ChunkCoord, &mut ChunkMeshingTask)>,
) {
    let commands = Arc::new(Mutex::new(commands));
    let materials = Arc::new(Mutex::new(materials));
    let meshes = Arc::new(Mutex::new(meshes));

    query.par_for_each_mut(512, |(entity, chunk_coord, mut task)| {
        if let Some(mesh) = future::block_on(future::poll_once(&mut task.0)) {
            commands
                .lock()
                .entity(entity)
                .insert(ChunkMeshed)
                .remove::<ChunkMeshingTask>();

            let Some(mesh) = mesh else {
                return;
            };

            let mesh = meshes.lock().add(mesh);
            let material = {
                let mut rng = rand::thread_rng();
                let mut m = StandardMaterial::from(Color::rgb(
                    rng.gen_range(0.0..=1.0),
                    rng.gen_range(0.0..=1.0),
                    rng.gen_range(0.0..=1.0),
                ));
                m.perceptual_roughness = 0.6;
                m.metallic = 0.2;
                materials.lock().add(m)
            };

            let chunk_min = chunk_coord.0 * UNPADDED_CHUNK_SHAPE;
            let transform = Transform::from_translation(chunk_min.as_vec3());

            commands.lock().entity(entity).insert(PbrBundle {
                mesh,
                material,
                transform,
                ..Default::default()
            });
        }
    });
}

#[derive(Reflect, Resource, Default)]
struct ChunkMapDebug {
    generation_tasks_count: usize,
    meshing_tasks_count: usize,
    generated_chunks_count: usize,
    meshed_chunks_count: usize,
}

fn debug_generation_tasks(
    mut debug: ResMut<ChunkMapDebug>,
    query: Query<(), (With<ChunkCoord>, With<ChunkGenerationTask>)>,
) {
    debug.generation_tasks_count = query.iter().len();
}

fn debug_meshing_tasks(
    mut debug: ResMut<ChunkMapDebug>,
    query: Query<(), (With<ChunkCoord>, With<ChunkMeshingTask>)>,
) {
    debug.meshing_tasks_count = query.iter().len();
}

fn debug_generated_chunks(
    mut debug: ResMut<ChunkMapDebug>,
    query: Query<(), (With<ChunkCoord>, With<ChunkGenerated>)>,
) {
    debug.generated_chunks_count = query.iter().len();
}

fn debug_meshed_chunks(
    mut debug: ResMut<ChunkMapDebug>,
    query: Query<(), (With<ChunkCoord>, With<ChunkMeshed>)>,
) {
    debug.meshed_chunks_count = query.iter().len();
}
