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
    pub fn insert(&mut self, key: ChunkKey, chunk: ChunkData) -> Option<ChunkData> {
        self.chunks.insert(key, chunk)
    }

    pub fn get(&self, key: ChunkKey) -> Option<&ChunkData> {
        self.chunks.get(&key)
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
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

#[derive(Resource, Default)]
pub struct DirtyChunks(HashSet<ChunkKey>);

impl DirtyChunks {
    pub fn insert(&mut self, key: ChunkKey) -> bool {
        self.0.insert(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ChunkKey> {
        self.0.iter()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}
