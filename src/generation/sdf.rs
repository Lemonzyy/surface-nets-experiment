use bevy::math::Vec3;

#[inline]
pub fn world(p: Vec3) -> f32 {
    // infinite_repetition(p, Vec3::splat(80.0), |q| sphere(q, 32.0))
    infinite_repetition(p, Vec3::splat(256.0), |q| sphere(q, 128.0))
    // sphere(p, 640.0)
}

#[inline]
fn sphere(p: Vec3, r: f32) -> f32 {
    p.length() - r
}

#[inline]
fn infinite_repetition(p: Vec3, c: Vec3, primitive: impl Fn(Vec3) -> f32) -> f32 {
    // q = mod(pf+0.5*c,c)-0.5*c;
    let q = modulo(p + 0.5 * c, c) - 0.5 * c;
    primitive(q)
}

// Others

/// From: https://registry.khronos.org/OpenGL-Refpages/gl4/html/mod.xhtml
#[inline]
fn modulo(x: Vec3, y: Vec3) -> Vec3 {
    x - y * (x / y).floor()
}
