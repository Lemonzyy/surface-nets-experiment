use bevy::prelude::*;

use crate::constants::*;

#[derive(Clone)]
pub struct ChunkData {
    pub sdf: [Sd8; UNPADDED_CHUNK_SIZE],
}

impl ChunkData {
    pub fn empty() -> Self {
        Self {
            sdf: [DEFAULT_SDF_VALUE; UNPADDED_CHUNK_SIZE],
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut, Reflect)]
pub struct ChunkKey(pub IVec3);

impl ChunkKey {
    // Minimum point of the chunk
    pub fn min_point(&self) -> IVec3 {
        self.0 * UNPADDED_CHUNK_SHAPE
    }

    /// Extent containing all the points of the chunk
    pub fn extent(&self) -> Extent3i {
        Extent3i::from_min_and_shape(self.min_point(), UNPADDED_CHUNK_SHAPE)
    }
}

impl From<IVec3> for ChunkKey {
    fn from(value: IVec3) -> Self {
        Self(value)
    }
}
