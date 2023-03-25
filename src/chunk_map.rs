use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::chunk::{ChunkData, ChunkKey};

#[derive(Resource, Default)]
pub struct ChunkMap {
    chunks: HashMap<ChunkKey, ChunkData>,
}

impl ChunkMap {
    pub fn insert_chunk(&mut self, key: ChunkKey, chunk: ChunkData) -> Option<ChunkData> {
        self.chunks.insert(key, chunk)
    }

    pub fn get_chunk(&self, key: &ChunkKey) -> Option<&ChunkData> {
        self.chunks.get(key)
    }
}

#[derive(Resource, Default)]
pub struct LoadedChunks(HashMap<ChunkKey, Entity>);

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
