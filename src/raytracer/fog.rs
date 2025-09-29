use raylib::core::math::Vector3;
use super::color::{lerp, clamp01};

/// Genera estrellas procedurales usando hash simple
fn star_brightness(dir: Vector3) -> f32 {
    if dir.y < 0.1 { return 0.0; } // No estrellas cerca del horizonte
    
    // Hash simple basado en dirección
    let scale = 50.0; // Densidad de estrellas
    let x = (dir.x * scale).floor() as i32;
    let z = (dir.z * scale).floor() as i32;
    
    // Hash simple para generar estrellas pseudo-aleatorias
    let hash = ((x.wrapping_mul(73856093)) ^ (z.wrapping_mul(19349663))) as u32;
    let star_chance = (hash % 1000) as f32 / 1000.0;
    
    if star_chance < 0.98 { return 0.0; } // Solo 2% de posiciones tienen estrellas
    
    // Brillo variable de las estrellas
    let brightness_hash = (hash >> 16) % 100;
    let brightness = 0.3 + (brightness_hash as f32 / 100.0) * 0.7; // Entre 0.3 y 1.0
    
    // Las estrellas son más brillantes cuando están más arriba
    let height_factor = (dir.y - 0.1) / 0.9;
    brightness * height_factor
}

/// Cielo (sRGB) con gradiente zenit↔horizonte - día/noche con estrellas
#[inline]
pub fn sky_srgb(dir: Vector3, is_night: bool) -> Vector3 {
    if is_night {
        // Colores de noche
        let zenith_night  = Vector3::new(0.02, 0.04, 0.12); // azul muy oscuro arriba
        let horizon_night = Vector3::new(0.06, 0.08, 0.20); // azul oscuro en horizonte
        let t = clamp01(dir.y * 0.5 + 0.5).powf(0.65);
        let mut sky_color = lerp(zenith_night, horizon_night, t);
        
        // Agregar estrellas
        let star_intensity = star_brightness(dir);
        if star_intensity > 0.0 {
            let star_color = Vector3::new(0.9, 0.9, 1.0); // Blanco azulado
            sky_color += star_color * star_intensity;
        }
        
        sky_color
    } else {
        // Colores de día (originales)
        let zenith  = Vector3::new(0.18, 0.37, 0.77); // arriba
        let horizon = Vector3::new(0.78, 0.86, 0.95); // horizonte
        let t = clamp01(dir.y * 0.5 + 0.5).powf(0.65);
        lerp(zenith, horizon, t)
    }
}
