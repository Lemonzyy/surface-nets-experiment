use bevy::prelude::*;
use bracket_noise::prelude::{FastNoise, FractalType, NoiseType};
use once_cell::sync::Lazy;
use tracing::instrument;

use super::sdf;
use crate::{
    chunk::{Chunk, ChunkKey, Sd8},
    LEVEL_OF_DETAIL,
};

pub static GENERATOR: Lazy<Generator> = Lazy::new(Default::default);

pub struct Generator {
    simplex_fractal_rigid_multi: FastNoise,
}

impl Default for Generator {
    fn default() -> Self {
        Generator {
            simplex_fractal_rigid_multi: {
                let mut n = FastNoise::new();
                n.set_seed(43210);
                n.set_noise_type(NoiseType::SimplexFractal);
                n.set_fractal_octaves(6);
                n.set_fractal_type(FractalType::RigidMulti);
                n
            },
        }
    }
}

impl Generator {
    #[instrument(skip_all, level = "trace")]
    pub fn generate_chunk(&self, key: ChunkKey) -> Chunk {
        let chunk_extent = key.extent();
        let mut chunk_data = Chunk::new_empty();

        chunk_extent.iter3().for_each(|p| {
            let offset = p - chunk_extent.minimum;
            let sd = Sd8::from(
                self.generate_signed_distance(p.as_vec3() * LEVEL_OF_DETAIL)
                    / LEVEL_OF_DETAIL,
            );

            chunk_data.set_voxel(offset, sd);
        });

        chunk_data
    }

    fn generate_signed_distance(&self, p: Vec3) -> f32 {
        // infinite_repetition(p, Vec3::splat(80.0), |q| sphere(q, 32.0))
        // infinite_repetition(p, Vec3::splat(256.0), |q| sphere(q, 128.0))
        // sphere(p, 640.0)

        const SPHERE_RADIUS: f32 = 260.0;
        const NOISE_FREQUENCY: f32 = 0.002;
        const NOISE_AMPLITUDE: f32 = 60.0;

        let projected_p = p.normalize() * SPHERE_RADIUS;
        let noise = self.generate_simplex_fractal_rigid_multi(NOISE_FREQUENCY * projected_p);
        let perturbed_radius = NOISE_AMPLITUDE * -noise + SPHERE_RADIUS;

        sdf::sphere(p, perturbed_radius)
    }

    fn generate_simplex_fractal_rigid_multi(&self, p: Vec3) -> f32 {
        let [x, y, z] = p.to_array();
        self.simplex_fractal_rigid_multi.get_noise3d(x, y, z)
    }
}
