use raylib::core::math::Vector3;
use super::color::{lerp, clamp01};

/// Cielo (sRGB) con gradiente zenit↔horizonte
#[inline]
pub fn sky_srgb(dir: Vector3) -> Vector3 {
    // Ajusta estos colores si lo quieres más claro/oscuro
    let zenith  = Vector3::new(0.18, 0.37, 0.77); // arriba
    let horizon = Vector3::new(0.78, 0.86, 0.95); // horizonte
    let t = clamp01(dir.y * 0.5 + 0.5).powf(0.65);
    lerp(zenith, horizon, t)
}
