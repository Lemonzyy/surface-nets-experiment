use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use float_ord::FloatOrd;

use crate::chunk::{ChunkData, ChunkKey};

#[derive(Resource, Default)]
pub struct ChunkMap {
    chunks: HashMap<ChunkKey, ChunkData>,
}

impl ChunkMap {
    pub fn insert_chunk(&mut self, key: ChunkKey, chunk: ChunkData) -> Option<ChunkData> {
        self.chunks.insert(key, chunk)
    }

    pub fn get_chunk(&self, key: ChunkKey) -> Option<&ChunkData> {
        self.chunks.get(&key)
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
pub struct LoadedChunks(HashMap<ChunkKey, Entity>);

impl LoadedChunks {
    pub fn insert(&mut self, key: ChunkKey, entity: Entity) {
        self.0.insert(key, entity);
    }

    pub fn get_entity(&self, key: ChunkKey) -> Option<Entity> {
        self.0.get(&key).copied()
    }
}

pub struct DirtyChunks(HashSet<ChunkKey>);

impl DirtyChunks {
    pub fn insert(&mut self, key: ChunkKey) -> bool {
        self.0.insert(key)
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &ChunkKey> {
        self.0.iter()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }
}
