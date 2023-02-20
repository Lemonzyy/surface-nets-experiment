use bevy::prelude::*;

use crate::chunk::Chunk;

#[derive(Resource, Default)]
pub struct ChunkMap {
    chunks: bevy::utils::HashMap<IVec3, Chunk>,
}

impl ChunkMap {
    pub fn insert_chunk(&mut self, coord: IVec3, chunk: Chunk) -> Option<Chunk> {
        self.chunks.insert(coord, chunk)
    }

    pub fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk> {
        self.chunks.get(coord)
    }
}
