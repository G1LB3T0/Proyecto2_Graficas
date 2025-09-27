use raylib::core::math::Vector3;

pub mod math {
    use super::Vector3;

    #[inline] pub fn clamp01(x: f32) -> f32 { if x < 0.0 { 0.0 } else if x > 1.0 { 1.0 } else { x } }

    #[inline] pub fn gamma_encode(c: Vector3) -> Vector3 {
        Vector3::new(c.x.powf(1.0/2.2), c.y.powf(1.0/2.2), c.z.powf(1.0/2.2))
    }

    // Esféricas <-> cartesianas (para la luz orbital)
    #[inline]
    pub fn sph_to_cart(yaw: f32, pitch: f32, r: f32, target: Vector3) -> Vector3 {
        let x = r * pitch.cos() * yaw.cos();
        let y = r * pitch.sin();
        let z = r * pitch.cos() * yaw.sin();
        Vector3::new(x, y, z) + target
    }

    #[inline]
    pub fn cart_to_sph(pos: Vector3, target: Vector3) -> (f32, f32, f32) {
        let v = pos - target;
        let r = v.length();
        let yaw = v.z.atan2(v.x);     // [-PI, PI]
        let pitch = (v.y / r).asin(); // [-PI/2, PI/2]
        (yaw, pitch, r)
    }
}

pub mod texture {
    use image::{RgbaImage, Rgba};

    pub fn make_checkerboard(size: u32, cell: u32) -> RgbaImage {
        let mut img = RgbaImage::new(size, size);
        for y in 0..size {
            for x in 0..size {
                let on = ((x / cell) ^ (y / cell)) & 1;
                let c = if on == 1 { 210 } else { 40 };
                img.put_pixel(x, y, Rgba([c, c, c, 255]));
            }
        }
        img
    }

    pub fn load_or_make_texture(path: &str) -> RgbaImage {
        match image::open(path) {
            Ok(img) => img.to_rgba8(),
            Err(_) => {
                eprintln!("WARN: no se encontró {path}, usando checkerboard procedural.");
                make_checkerboard(256, 16)
            }
        }
    }
}
