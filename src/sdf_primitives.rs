use bevy::math::Vec3;

// Adapted from: https://iquilezles.org/articles/distfunctions/

pub fn sphere(p: Vec3, r: f32) -> f32 {
    p.length() - r
}

pub fn infinite_repetition(p: Vec3, c: Vec3, primitive: impl Fn(Vec3) -> f32) -> f32 {
    // q = mod(pf+0.5*c,c)-0.5*c;
    let q = modulo(p + 0.5 * c, c) - 0.5 * c;
    primitive(q)
}

// Others

// Adapted from: https://registry.khronos.org/OpenGL-Refpages/gl4/html/mod.xhtml
fn modulo(x: Vec3, y: Vec3) -> Vec3 {
    x - y * (x / y).floor()
}
