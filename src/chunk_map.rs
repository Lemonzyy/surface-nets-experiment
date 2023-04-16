use std::vec::Drain;

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
    pub storage: Storage,
}

#[derive(Default)]
pub struct Storage(HashMap<ChunkKey, ChunkData>);

impl Storage {
    pub fn insert(&mut self, key: ChunkKey, chunk: ChunkData) {
        self.0.insert(key, chunk);
    }

    pub fn get(&self, key: ChunkKey) -> Option<&ChunkData> {
        self.0.get(&key)
    }

    pub fn contains(&self, key: ChunkKey) -> bool {
        self.0.contains_key(&key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
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

pub fn copy_chunk_neighborhood(storage: &Storage, key: ChunkKey) -> [Sd8; PADDED_CHUNK_SIZE] {
    let padded_chunk_extent = key.extent().padded(1);
    let mut neighborhood = [Sd8::default(); PADDED_CHUNK_SIZE];

    chunks_in_extent(&padded_chunk_extent)
        .filter_map(|chunk_key| {
            let chunk_extent = chunk_key.extent();
            let intersection = padded_chunk_extent.intersection(&chunk_extent);

            storage
                .get(chunk_key)
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
