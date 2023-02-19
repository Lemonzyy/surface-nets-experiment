use bevy::prelude::*;

use crate::{chunk::Chunk, constants::*};

#[derive(Resource)]
pub struct ChunkMap {
    pub chunks: bevy::utils::HashMap<IVec3, Chunk>,
    sdf: fn(IVec3) -> Sd8,
}

impl ChunkMap {
    pub fn from_sdf(sdf: fn(IVec3) -> Sd8) -> Self {
        ChunkMap { sdf, ..default() }
    }

    pub fn generate_voxel(&self, coord: IVec3) -> Sd8 {
        (self.sdf)(coord)
    }

    pub fn insert_chunk(&mut self, coord: IVec3, chunk: Chunk) -> Option<Chunk> {
        self.chunks.insert(coord, chunk)
    }

    pub fn get_chunk(&self, coord: &IVec3) -> Option<&Chunk> {
        self.chunks.get(coord)
    }
}

impl Default for ChunkMap {
    fn default() -> Self {
        Self {
            chunks: Default::default(),
            sdf: |_| DEFAULT_SDF_VALUE,
        }
    }
}
