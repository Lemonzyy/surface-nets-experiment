use bevy::prelude::*;
use fast_surface_nets::{
    ndshape::{ConstShape, ConstShape3u32},
    SignedDistance,
};
use ilattice::prelude::*;

pub type Extent3i = Extent<IVec3>;

pub const CHUNK_SIDE: u32 = 32;
pub const CHUNK_SHAPE: IVec3 = IVec3::splat(CHUNK_SIDE as i32);
pub type ChunkShape = ConstShape3u32<CHUNK_SIDE, CHUNK_SIDE, CHUNK_SIDE>;
pub const CHUNK_SHAPE_LOG2: IVec3 = IVec3::splat(5);
pub const CHUNK_SIZE: usize = ChunkShape::SIZE as usize;

pub const PADDED_CHUNK_SIDE: u32 = CHUNK_SIDE + 2;
pub const PADDED_CHUNK_SHAPE: IVec3 = IVec3::splat(PADDED_CHUNK_SIDE as i32);
pub type PaddedChunkShape = ConstShape3u32<PADDED_CHUNK_SIDE, PADDED_CHUNK_SIDE, PADDED_CHUNK_SIDE>;
pub const PADDED_CHUNK_SIZE: usize = PaddedChunkShape::SIZE as usize;

#[derive(Debug, Clone, Copy, Reflect, FromReflect)]
pub struct Sd8(pub i8);

impl Sd8 {
    pub const RESOLUTION: f32 = i8::MAX as f32;
    pub const PRECISION: f32 = 1.0 / Self::RESOLUTION;
    pub const MAX: Self = Sd8(i8::MAX);
}

impl From<Sd8> for f32 {
    fn from(d: Sd8) -> Self {
        d.0 as f32 * Sd8::PRECISION
    }
}

impl From<f32> for Sd8 {
    fn from(d: f32) -> Self {
        Self((Self::RESOLUTION * d.min(1.0).max(-1.0)) as i8)
    }
}

impl SignedDistance for Sd8 {
    fn is_negative(self) -> bool {
        self.0 < 0
    }
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub sdf: [Sd8; CHUNK_SIZE],
}

impl Chunk {
    pub fn new_empty() -> Self {
        Self {
            sdf: [Sd8::MAX; CHUNK_SIZE],
        }
    }

    pub fn set_voxel(&mut self, offset: IVec3, sd: Sd8) {
        let index = ChunkShape::linearize(offset.as_uvec3().to_array()) as usize;
        self.sdf[index] = sd;
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut, Reflect)]
pub struct ChunkKey(pub IVec3);

impl ChunkKey {
    // Minimum point of the chunk
    pub fn min_point(&self) -> IVec3 {
        self.0 * CHUNK_SHAPE
    }

    /// Extent containing all the points of the chunk
    pub fn extent(&self) -> Extent3i {
        Extent3i::from_min_and_shape(self.min_point(), CHUNK_SHAPE)
    }
}

impl From<IVec3> for ChunkKey {
    fn from(value: IVec3) -> Self {
        Self(value)
    }
}
