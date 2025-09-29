use raylib::core::math::Vector3;
use super::color::{lerp, clamp01};

/// Cielo (sRGB) con gradiente zenit↔horizonte - día/noche
#[inline]
pub fn sky_srgb(dir: Vector3, is_night: bool) -> Vector3 {
    if is_night {
        // Colores de noche
        let zenith_night  = Vector3::new(0.02, 0.04, 0.12); // azul muy oscuro arriba
        let horizon_night = Vector3::new(0.06, 0.08, 0.20); // azul oscuro en horizonte
        let t = clamp01(dir.y * 0.5 + 0.5).powf(0.65);
        lerp(zenith_night, horizon_night, t)
    } else {
        // Colores de día (originales)
        let zenith  = Vector3::new(0.18, 0.37, 0.77); // arriba
        let horizon = Vector3::new(0.78, 0.86, 0.95); // horizonte
        let t = clamp01(dir.y * 0.5 + 0.5).powf(0.65);
        lerp(zenith, horizon, t)
    }
}
