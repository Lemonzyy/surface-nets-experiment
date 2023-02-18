use bevy::prelude::*;

use crate::constants::Sd8;

#[derive(Clone, Default)]
pub struct Chunk {
    pub data: Vec<Sd8>,
    pub entity: Option<Entity>,
}

#[derive(Component, Reflect, Debug)]
pub struct ChunkCoord(pub IVec3);

#[derive(Component)]
pub struct ChunkGenerated;

#[derive(Component)]
pub struct ChunkMeshed;
