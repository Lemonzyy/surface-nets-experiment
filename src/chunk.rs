use bevy::prelude::*;

use crate::constants::*;

#[derive(Clone)]
pub struct Chunk {

    pub data: [Sd8; UNPADDED_CHUNK_SIZE],
    pub entity: Option<Entity>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            data: [DEFAULT_SDF_VALUE; UNPADDED_CHUNK_SIZE],
            entity: None,
        }
    }
}

#[derive(Component, Reflect, Debug)]
pub struct ChunkCoord(pub IVec3);

#[derive(Component)]
pub struct ChunkGenerated;

#[derive(Component)]
pub struct ChunkMeshed;
