use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    utils::HashMap,
};
use fast_surface_nets::{
    ndshape::{ConstShape, ConstShape3u32},
    surface_nets, SignedDistance, SurfaceNetsBuffer,
};
use ilattice::prelude::Extent;
use noise::{MultiFractal, NoiseFn, RidgedMulti, Simplex};
use rand::Rng;

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

#[derive(Debug, Clone, Copy, Reflect, FromReflect)]
struct Sd8(pub i8);

impl Sd8 {
    const RESOLUTION: f32 = i8::MAX as f32;
    const PRECISION: f32 = 1.0 / Self::RESOLUTION;
}

impl From<Sd8> for f32 {
    fn from(d: Sd8) -> Self {
        d.0 as f32 * Sd8::PRECISION
    }
}

impl From<f32> for Sd8 {
    fn from(d: f32) -> Self {
        Self((Self::RESOLUTION * d.min(1.0).max(-1.0)) as i8)
    }
}

impl SignedDistance for Sd8 {
    fn is_negative(self) -> bool {
        self.0 < 0
    }
}

const UNPADDED_CHUNK_SIDE: u32 = 32;
const UNPADDED_CHUNK_SHAPE: IVec3 = IVec3::splat(UNPADDED_CHUNK_SIDE as i32);
type UnpaddedChunkShape =
    ConstShape3u32<UNPADDED_CHUNK_SIDE, UNPADDED_CHUNK_SIDE, UNPADDED_CHUNK_SIDE>;

const CHUNK_PADDING: u32 = 1;
const PADDED_CHUNK_SIDE: u32 = UNPADDED_CHUNK_SIDE + 2 * CHUNK_PADDING;
const PADDED_CHUNK_SHAPE: IVec3 = IVec3::splat(PADDED_CHUNK_SIDE as i32);
type PaddedChunkShape = ConstShape3u32<PADDED_CHUNK_SIDE, PADDED_CHUNK_SIDE, PADDED_CHUNK_SIDE>;

const DEFAULT_SDF_VALUE: Sd8 = Sd8(i8::MAX);

type Extent3i = Extent<IVec3>;

#[derive(Clone, Default)]
struct Chunk {
    data: Vec<Sd8>,
    entity: Option<Entity>,
}

#[derive(Component, Reflect, Debug)]
struct ChunkCoord(IVec3);

#[derive(Component)]
struct Generated;

#[derive(Component)]
struct Meshed;

#[derive(Resource, Default)]
struct ChunkWorld {
    chunks: HashMap<IVec3, Chunk>,
}

fn setup(mut commands: Commands) {
    let chunks_extent =
        Extent3i::from_min_and_lub(IVec3::from([-10, -2, -10]), IVec3::from_array([10, 2, 10]));

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
    query: Query<(Entity, &ChunkCoord), Without<Generated>>,
) {
    for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;
        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let unpadded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, UNPADDED_CHUNK_SHAPE);

        let freq = 0.004;
        let ampl = 50.0;
        let noise = RidgedMulti::<Simplex>::default().set_frequency(freq);

        let mut unpadded_chunk_data = [DEFAULT_SDF_VALUE; UnpaddedChunkShape::SIZE as usize];

        unpadded_chunk_extent.iter3().for_each(|p| {
            let p_in_chunk = p - unpadded_chunk_extent.minimum;

            let v = &mut unpadded_chunk_data
                [UnpaddedChunkShape::linearize(p_in_chunk.as_uvec3().to_array()) as usize];

            //*v = sdf(p);
            *v = (p.y as f32 - (noise.get(p.as_dvec3().to_array()) * ampl) as f32).into();
        });

        chunk_world.chunks.insert(
            chunk_coord,
            Chunk {
                data: unpadded_chunk_data.to_vec(),
                entity: Some(entity),
            },
        );

        commands.entity(entity).insert(Generated);
    }
}

const ADJACENT_CHUNKS_OFFSET: [IVec3; 7] = [
    IVec3::new(0, 0, 1),
    IVec3::new(0, 1, 0),
    IVec3::new(0, 1, 1),
    IVec3::new(1, 0, 0),
    IVec3::new(1, 0, 1),
    IVec3::new(1, 1, 0),
    IVec3::new(1, 1, 1),
];

fn generate_meshes(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_world: Res<ChunkWorld>,
    query: Query<(Entity, &ChunkCoord), (With<Generated>, Without<Meshed>)>,
) {
    let mut buffer = SurfaceNetsBuffer::default();
    let mut color_rng = rand::thread_rng();

    for (entity, chunk_coord) in &query {
        let chunk_coord = chunk_coord.0;
        let Some(chunk) = chunk_world.chunks.get(&chunk_coord) else {
            continue;
        };

        let chunk_min = chunk_coord * UNPADDED_CHUNK_SHAPE;
        let padded_chunk_extent = Extent3i::from_min_and_shape(chunk_min, PADDED_CHUNK_SHAPE);

        let mut samples = [DEFAULT_SDF_VALUE; PaddedChunkShape::SIZE as usize];

        let adjacent_chunks = ADJACENT_CHUNKS_OFFSET.map(|offset| chunk_coord + offset);

        let adjacent_chunks_intersection_extent = adjacent_chunks
            .map(|adj_chunk| adj_chunk * UNPADDED_CHUNK_SHAPE)
            .map(|adj_chunk_min| Extent3i::from_min_and_shape(adj_chunk_min, UNPADDED_CHUNK_SHAPE))
            .map(|adj_chunk_ext| padded_chunk_extent.intersection(&adj_chunk_ext));

        ADJACENT_CHUNKS_OFFSET
            .into_iter()
            .zip(adjacent_chunks.into_iter())
            .zip(adjacent_chunks_intersection_extent)
            .for_each(|((offset, chunk), intersection_extent)| {
                if let Some(chunk) = chunk_world.chunks.get(&chunk) {
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

        ndcopy::copy3(
            UnpaddedChunkShape::ARRAY,
            &chunk.data,
            &UnpaddedChunkShape {},
            [0; 3],
            &mut samples,
            &PaddedChunkShape {},
            [0; 3],
        ); // TODO put chunk offset 0,0,0 to remove this

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

        commands.entity(entity).insert(Meshed);
    }
}

/*                 if print {
    let transform = Transform::from_translation(
        (offset * UNPADDED_CHUNK_SHAPE).as_vec3()
            + 0.5 * intersection_ext.shape.as_vec3(),
    );

    let e = commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box::new(
                intersection_ext.shape.x as f32,
                intersection_ext.shape.y as f32,
                intersection_ext.shape.z as f32,
            ))),
            material: materials.add({
                let mut m = StandardMaterial::from(Color::rgba(
                    color_rng.gen_range(0.0..=1.0),
                    color_rng.gen_range(0.0..=1.0),
                    color_rng.gen_range(0.0..=1.0),
                    0.5,
                ));
                m.alpha_mode = AlphaMode::Blend;
                m
            }),
            transform,
            ..Default::default()
        })
        .id();
    commands.entity(entity).add_child(e);
} */
