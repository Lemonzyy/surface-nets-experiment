use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::chunk::ChunkData;

#[derive(Resource, Default)]
pub struct ChunkMap {
    chunks: HashMap<IVec3, ChunkData>,
}

impl ChunkMap {
    pub fn insert_chunk(&mut self, key: IVec3, chunk: ChunkData) -> Option<ChunkData> {
        self.chunks.insert(key, chunk)
    }

    pub fn get_chunk(&self, key: &IVec3) -> Option<&ChunkData> {
        self.chunks.get(key)
    }
}

#[derive(Resource, Default)]
pub struct LoadedChunks(HashMap<IVec3, Entity>);

pub struct DirtyChunks(HashSet<IVec3>);

impl DirtyChunks {
    pub fn insert(&mut self, key: IVec3) -> bool {
        self.0.insert(key)
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &IVec3> {
        self.0.iter()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }
}
