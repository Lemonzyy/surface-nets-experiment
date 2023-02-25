use bevy::prelude::*;

use crate::chunk::Chunk;

#[derive(Resource, Default)]
pub struct ChunkMap {
    chunks: bevy::utils::HashMap<IVec3, Chunk>,
    pending_chunks: bevy::utils::HashSet<IVec3>,
}

impl ChunkMap {
    pub fn insert_chunk(&mut self, coord: IVec3, chunk: Chunk) -> Option<Chunk> {
        self.chunks.insert(coord, chunk)
    }

    pub fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk> {
        self.chunks.get(coord)
    }

    pub fn insert_pending_chunk(&mut self, coord: IVec3) -> bool {
        self.pending_chunks.insert(coord)
    }

    pub fn remove_pending_chunk(&mut self, coord: &IVec3) -> bool {
        self.pending_chunks.remove(coord)
    }

    pub fn is_pending_chunk(&self, coord: &IVec3) -> bool {
        self.pending_chunks.contains(coord)
    }
}
