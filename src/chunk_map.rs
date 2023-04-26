use std::vec::Drain;

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use float_ord::FloatOrd;
use tracing::instrument;

use crate::chunk::{
    Chunk, ChunkKey, ChunkShape, Extent3i, PaddedChunkShape, Sd8, CHUNK_SHAPE_LOG2,
    PADDED_CHUNK_SHAPE, PADDED_CHUNK_SIZE,
};

#[derive(Resource, Default)]
pub struct ChunkMap {
    pub storage: HashMap<ChunkKey, Chunk>,
}

impl ChunkMap {
    #[instrument(skip_all, level = "trace")]
    pub fn copy_chunk_neighborhood(&self, key: ChunkKey) -> [Sd8; PADDED_CHUNK_SIZE] {
        let padded_chunk_extent = key.extent().with_shape(PADDED_CHUNK_SHAPE);
        let mut neighborhood = [Sd8::MAX; PADDED_CHUNK_SIZE];

        chunks_in_extent(&padded_chunk_extent)
            .filter_map(|chunk_key| {
                let chunk_extent = chunk_key.extent();
                let intersection = padded_chunk_extent.intersection(&chunk_extent);

                self.storage
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
                    &ChunkShape {},
                    src_start,
                    &mut neighborhood,
                    &PaddedChunkShape {},
                    dst_start,
                );
            });

        neighborhood
    }
}

#[derive(Resource, Debug, Default)]
pub struct ChunkCommandQueue {
    create: Vec<ChunkKey>,
    delete: Vec<ChunkKey>,
}

impl ChunkCommandQueue {
    pub fn push(&mut self, command: ChunkCommand) {
        match command {
            ChunkCommand::Create(key) => self.create.push(key),
            ChunkCommand::Delete(key) => self.delete.push(key),
        }
    }

    pub fn sort_by_distance(&mut self, key: ChunkKey) {
        self.create
            .sort_unstable_by_key(|k| FloatOrd(k.as_vec3().distance_squared(key.as_vec3())));
    }

    pub fn is_create_empty(&self) -> bool {
        self.create.is_empty()
    }

    pub fn is_delete_empty(&self) -> bool {
        self.delete.is_empty()
    }

    pub fn create_len(&self) -> usize {
        self.create.len()
    }

    pub fn delete_len(&self) -> usize {
        self.delete.len()
    }

    pub fn drain_create_commands(&mut self) -> Drain<ChunkKey> {
        self.create.drain(..)
    }

    pub fn drain_delete_commands(&mut self) -> Drain<ChunkKey> {
        self.delete.drain(..)
    }
}

pub enum ChunkCommand {
    Create(ChunkKey),
    Delete(ChunkKey),
}

#[derive(Resource, Default)]
pub struct CurrentChunks(HashMap<ChunkKey, Entity>);

impl CurrentChunks {
    pub fn add(&mut self, key: ChunkKey, entity: Entity) {
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

pub fn chunks_in_extent(extent: &Extent3i) -> impl Iterator<Item = ChunkKey> {
    let range_min = extent.minimum >> CHUNK_SHAPE_LOG2;
    let range_max = extent.max() >> CHUNK_SHAPE_LOG2;

    Extent3i::from_min_and_max(range_min, range_max)
        .iter3()
        .map(ChunkKey::from)
}
