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

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut, Reflect)]
pub struct ChunkKey(pub IVec3);

impl From<IVec3> for ChunkKey {
    fn from(value: IVec3) -> Self {
        Self(value)
    }
}

#[derive(Component)]
pub struct NeedGenerating;

#[derive(Component)]
pub struct NeedMeshing;
