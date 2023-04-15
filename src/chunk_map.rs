use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use float_ord::FloatOrd;

use crate::{
    chunk::{ChunkData, ChunkKey},
    constants::*,
};

#[derive(Resource, Default)]
pub struct ChunkMap {
    pub storage: HashMap<ChunkKey, ChunkData>,
}

impl ChunkMap {
    pub fn insert(&mut self, key: ChunkKey, chunk: ChunkData) -> Option<ChunkData> {
        self.storage.insert(key, chunk)
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }
}

#[derive(Resource, Debug, Default)]
pub struct ChunkCommandQueue {
    pub create: Vec<ChunkKey>,
    pub delete: Vec<ChunkKey>,
}

impl ChunkCommandQueue {
    pub fn sort(&mut self, center: ChunkKey) {
        self.create
            .sort_unstable_by_key(|k| FloatOrd(k.as_vec3().distance_squared(center.as_vec3())));
    }
}

#[derive(Resource, Default)]
pub struct ChunkEntityRelation(HashMap<ChunkKey, Entity>);

impl ChunkEntityRelation {
    pub fn link(&mut self, key: ChunkKey, entity: Entity) {
        self.0.insert(key, entity);
    }

    pub fn get_entity(&self, key: ChunkKey) -> Option<Entity> {
        self.0.get(&key).copied()
    }

    pub fn contains(&self, key: ChunkKey) -> bool {
        self.0.contains_key(&key)
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DirtyChunks(HashSet<ChunkKey>);

pub fn copy_chunk_neighborhood(
    chunks: &HashMap<ChunkKey, ChunkData>,
    key: ChunkKey,
) -> [Sd8; PADDED_CHUNK_SIZE] {
    let padded_chunk_extent = key.extent().padded(1);
    let mut neighborhood = [Sd8::default(); PADDED_CHUNK_SIZE];

    chunks_in_extent(&padded_chunk_extent)
        .filter_map(|chunk_key| {
            let chunk_extent = chunk_key.extent();
            let intersection = padded_chunk_extent.intersection(&chunk_extent);

            chunks
                .get(&chunk_key)
                .map(|chunk| (chunk_key, intersection, chunk.sdf))
        })
        .map(|(chunk_key, extent, sdf)| {
            let copy_shape = extent.shape.as_uvec3().to_array();
            let src_start = (extent.minimum - chunk_key.min_point())
                .as_uvec3()
                .to_array();
            let dst_start = (extent.minimum - padded_chunk_extent.minimum)
                .as_uvec3()
                .to_array();
            (copy_shape, sdf, src_start, dst_start)
        })
        .for_each(|(copy_shape, sdf, src_start, dst_start)| {
            ndcopy::copy3(
                copy_shape,
                &sdf,
                &UnpaddedChunkShape {},
                src_start,
                &mut neighborhood,
                &PaddedChunkShape {},
                dst_start,
            );
        });

    neighborhood
}

pub fn chunks_in_extent(extent: &Extent3i) -> impl Iterator<Item = ChunkKey> {
    let range_min = extent.minimum >> UNPADDED_CHUNK_SHAPE_LOG2;
    let range_max = extent.max() >> UNPADDED_CHUNK_SHAPE_LOG2;

    Extent3i::from_min_and_max(range_min, range_max)
        .iter3()
        .map(ChunkKey::from)
}
