use bevy::math::Vec3A;

// Adapted from: https://iquilezles.org/articles/distfunctions/

pub fn sphere(p: Vec3A, r: f32) -> f32 {
    p.length() - r
}

pub fn infinite_repetition(p: Vec3A, c: Vec3A, primitive: impl Fn(Vec3A) -> f32) -> f32 {
    // q = mod(pf+0.5*c,c)-0.5*c;
    let q = modulo(p + 0.5 * c, c) - 0.5 * c;
    primitive(q)
}

// Others

// Adapted from: https://registry.khronos.org/OpenGL-Refpages/gl4/html/mod.xhtml
fn modulo(x: Vec3A, y: Vec3A) -> Vec3A {
    x - y * (x / y).floor()
}
