use bevy::prelude::*;

use crate::constants::*;

#[derive(Clone)]
pub struct ChunkData {
    pub sdf: [Sd8; UNPADDED_CHUNK_SIZE],
}

impl Default for ChunkData {
    fn default() -> Self {
        Self {
            sdf: [DEFAULT_SDF_VALUE; UNPADDED_CHUNK_SIZE],
        }
    }
}

#[derive(Component, Reflect, Debug)]
pub struct Chunk {
    pub key: IVec3,
}

#[derive(Component)]
pub struct NeedGenerating;

#[derive(Component)]
pub struct NeedMeshing;
