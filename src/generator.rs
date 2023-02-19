use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
};
use fast_surface_nets::{ndshape::ConstShape, surface_nets, SurfaceNetsBuffer};
use ilattice::vector::ZipMap;
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
            .insert_resource(ChunkMap::from_sdf(|p| {
                let pf = p.as_vec3();
                let c = Vec3::new(68.0, 68.0, 68.0);
                // q = mod(pf+0.5*c,c)-0.5*c;
                let q = (pf + 0.5 * c).zip_map(c, |c1, c2| c1 % c2) - 0.5 * c;

                fn sphere(p: Vec3) -> Sd8 {
                    (p.length() - 32.0).into()
                }

                sphere(q)
            }))
            .add_startup_system(spawn_chunks)
            .add_system(generate_chunks)
            .add_system(generate_chunk_meshes);
    }
}

fn spawn_chunks(mut commands: Commands) {
    let chunks_extent = Extent3i::from_min_and_lub(IVec3::splat(-5), IVec3::splat(5));

    chunks_extent
        .iter3()
        .for_each(|c| spawn_chunk(c, &mut commands));

    let chunk_count = chunks_extent.num_points();
    let voxel_count = chunk_count * (UNPADDED_CHUNK_SIZE as u64);
    info!(
        "Spawning {chunk_count} chunks ({}x{}x{}) for a total of {voxel_count} voxels",
        UnpaddedChunkShape::ARRAY[0],
        UnpaddedChunkShape::ARRAY[1],
        UnpaddedChunkShape::ARRAY[2],
    );
}

fn spawn_chunk(coord: IVec3, commands: &mut Commands) {
    commands.spawn((
        Name::new(format!(
            "Chunk {{ x:{}, y:{}, z:{} }}",
            coord.x, coord.y, coord.z
        )),
        ChunkCoord(coord),
    ));
}

fn generate_chunks(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    query: Query<(Entity, &ChunkCoord), Without<ChunkGenerated>>,
) {
    for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;
        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let unpadded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, UNPADDED_CHUNK_SHAPE);

        let mut chunk = Chunk {
            entity: Some(entity),
            ..default()
        };

        unpadded_chunk_extent.iter3().for_each(|p| {
            let p_in_chunk = p - unpadded_chunk_extent.minimum;

            let v = &mut chunk.data
                [UnpaddedChunkShape::linearize(p_in_chunk.as_uvec3().to_array()) as usize];

            *v = chunk_map.generate_voxel(p);
        });

        chunk_map.insert_chunk(chunk_coord, chunk);

        commands.entity(entity).insert(ChunkGenerated);
    }
}

#[allow(clippy::type_complexity)]
fn generate_chunk_meshes(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_map: Res<ChunkMap>,
    query: Query<(Entity, &ChunkCoord), (With<ChunkGenerated>, Without<ChunkMeshed>)>,
) {
    let mut buffer = SurfaceNetsBuffer::default();
    let mut color_rng = rand::thread_rng();

    for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;

        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let padded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, PADDED_CHUNK_SHAPE);

        let mut samples = [DEFAULT_SDF_VALUE; PADDED_CHUNK_SIZE];

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
