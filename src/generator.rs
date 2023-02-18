use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
};
use fast_surface_nets::{ndshape::ConstShape, surface_nets, SurfaceNetsBuffer};
use noise::{MultiFractal, NoiseFn, RidgedMulti, Seedable, Simplex};
use rand::Rng;

use crate::{
    chunk::{Chunk, ChunkCoord, ChunkGenerated, ChunkMeshed},
    chunk_world::ChunkWorld,
    constants::*,
};

pub struct GeneratorPlugin;

impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ChunkCoord>()
            .init_resource::<ChunkWorld>()
            .add_startup_system(setup)
            .add_system(generate_chunks)
            .add_system(generate_meshes);
    }
}

fn setup(mut commands: Commands) {
    let chunks_extent =
        Extent3i::from_min_and_lub(IVec3::from([-4, -2, -4]), IVec3::from_array([4, 2, 4]));

    chunks_extent.iter3().for_each(|coord| {
        commands.spawn((
            Name::new(format!(
                "Chunk {{ x:{}, y:{}, z:{} }}",
                coord.x, coord.y, coord.z
            )),
            ChunkCoord(coord),
        ));
    });

    let chunk_count = chunks_extent.num_points();
    let voxel_count = chunk_count * (UnpaddedChunkShape::SIZE as u64);
    info!(
        "Spawning {chunk_count} chunks ({}x{}x{}) for a total of {voxel_count} voxels",
        UnpaddedChunkShape::ARRAY[0],
        UnpaddedChunkShape::ARRAY[1],
        UnpaddedChunkShape::ARRAY[2],
    );
}

fn sdf(p: IVec3) -> Sd8 {
    const SPHERE_RADIUS: f32 = 150.0;
    (p.as_vec3a().length() - SPHERE_RADIUS).into()
}

fn generate_chunks(
    mut commands: Commands,
    mut chunk_world: ResMut<ChunkWorld>,
    query: Query<(Entity, &ChunkCoord), Without<ChunkGenerated>>,
) {
    for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;
        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let unpadded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, UNPADDED_CHUNK_SHAPE);

        let freq = 0.004;
        let ampl = 50.0;
        let noise = RidgedMulti::<Simplex>::default()
            .set_frequency(freq)
            .set_seed(442);

        let mut unpadded_chunk_data = [DEFAULT_SDF_VALUE; UnpaddedChunkShape::SIZE as usize];

        unpadded_chunk_extent.iter3().for_each(|p| {
            let p_in_chunk = p - unpadded_chunk_extent.minimum;

            let v = &mut unpadded_chunk_data
                [UnpaddedChunkShape::linearize(p_in_chunk.as_uvec3().to_array()) as usize];

            //*v = sdf(p);
            *v = (p.y as f32 - (noise.get(p.as_dvec3().to_array()) * ampl) as f32).into();
        });

        chunk_world.insert_chunk(
            chunk_coord,
            Chunk {
                data: unpadded_chunk_data.to_vec(),
                entity: Some(entity),
            },
        );

        commands.entity(entity).insert(ChunkGenerated);
    }
}

#[allow(clippy::type_complexity)]
fn generate_meshes(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_world: Res<ChunkWorld>,
    query: Query<(Entity, &ChunkCoord), (With<ChunkGenerated>, Without<ChunkMeshed>)>,
) {
    let mut buffer = SurfaceNetsBuffer::default();
    let mut color_rng = rand::thread_rng();

    for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;

        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let padded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, PADDED_CHUNK_SHAPE);

        let mut samples = [DEFAULT_SDF_VALUE; PaddedChunkShape::SIZE as usize];

        let meshing_chunks = MESHING_CHUNKS_OFFSET.map(|offset| chunk_coord + offset);

        let meshing_chunk_intersection_extents = meshing_chunks
            .map(|chunk| chunk * UNPADDED_CHUNK_SHAPE)
            .map(|chunk_min| Extent3i::from_min_and_shape(chunk_min, UNPADDED_CHUNK_SHAPE))
            .map(|adj_chunk_ext| padded_chunk_extent.intersection(&adj_chunk_ext));

        MESHING_CHUNKS_OFFSET
            .into_iter()
            .zip(meshing_chunks.into_iter())
            .zip(meshing_chunk_intersection_extents)
            .for_each(|((offset, chunk), intersection_extent)| {
                if let Some(chunk) = chunk_world.get_chunk(&chunk) {
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

            mesh.duplicate_vertices();
            mesh.compute_flat_normals();

            let mesh = meshes.add(mesh);
            let material = {
                let mut m = StandardMaterial::from(Color::rgb(
                    color_rng.gen_range(0.0..=1.0),
                    color_rng.gen_range(0.0..=1.0),
                    color_rng.gen_range(0.0..=1.0),
                ));
                m.perceptual_roughness = 0.6;
                m.metallic = 0.2;
                materials.add(m)
            };

            let transform = Transform::from_translation(chunk_min.as_vec3());

            commands.entity(entity).insert(PbrBundle {
                mesh,
                material,
                transform,
                ..Default::default()
            });
        }

        commands.entity(entity).insert(ChunkMeshed);
    }
}
