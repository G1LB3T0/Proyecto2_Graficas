use raylib::core::math::Vector3;

#[inline]
pub fn clamp01(x: f32) -> f32 { if x < 0.0 { 0.0 } else if x > 1.0 { 1.0 } else { x } }

#[inline]
pub fn srgb_to_linear(c: Vector3) -> Vector3 {
    // Aprox rápida
    Vector3::new(c.x.powf(2.2), c.y.powf(2.2), c.z.powf(2.2))
}

#[inline]
pub fn gamma_encode(c: Vector3) -> Vector3 {
    Vector3::new(c.x.powf(1.0/2.2), c.y.powf(1.0/2.2), c.z.powf(1.0/2.2))
}

#[inline]
pub fn lerp(a: Vector3, b: Vector3, t: f32) -> Vector3 { a * (1.0 - t) + b * t }
