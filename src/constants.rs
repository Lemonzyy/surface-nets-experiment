use bevy::prelude::*;
use fast_surface_nets::{
    ndshape::{ConstShape, ConstShape3u32},
    SignedDistance,
};
use ilattice::prelude::*;

pub type Extent3i = Extent<IVec3>;

pub const UNPADDED_CHUNK_SIDE: u32 = 32;
pub const UNPADDED_CHUNK_SHAPE: IVec3 = IVec3::splat(UNPADDED_CHUNK_SIDE as i32);
pub type UnpaddedChunkShape =
    ConstShape3u32<UNPADDED_CHUNK_SIDE, UNPADDED_CHUNK_SIDE, UNPADDED_CHUNK_SIDE>;
pub const UNPADDED_CHUNK_SHAPE_LOG2: IVec3 = IVec3::splat(5);
pub const UNPADDED_CHUNK_SIZE: usize = UnpaddedChunkShape::SIZE as usize;

pub const CHUNK_PADDING: u32 = 1;
pub const PADDED_CHUNK_SIDE: u32 = UNPADDED_CHUNK_SIDE + 2 * CHUNK_PADDING;
pub const PADDED_CHUNK_SHAPE: IVec3 = IVec3::splat(PADDED_CHUNK_SIDE as i32);
pub type PaddedChunkShape = ConstShape3u32<PADDED_CHUNK_SIDE, PADDED_CHUNK_SIDE, PADDED_CHUNK_SIDE>;
pub const PADDED_CHUNK_SIZE: usize = PaddedChunkShape::SIZE as usize;

pub const DEFAULT_SDF_VALUE: Sd8 = Sd8::MAX;

#[derive(Debug, Clone, Copy, Reflect, FromReflect)]
pub struct Sd8(pub i8);

impl Sd8 {
    const RESOLUTION: f32 = i8::MAX as f32;
    const PRECISION: f32 = 1.0 / Self::RESOLUTION;
    const MAX: Self = Sd8(i8::MAX);
}

impl Default for Sd8 {
    fn default() -> Self {
        DEFAULT_SDF_VALUE
    }
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
