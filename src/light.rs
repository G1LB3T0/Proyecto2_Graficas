use raylib::prelude::*;
use raylib::consts::KeyboardKey::*;

/// Control simple de luz orbital alrededor de `target`
pub struct LightRig {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub target: Vector3,
    pub spin: bool,
    pub min_radius: f32, // ← campo público para fijar radio mínimo
}

impl LightRig {
    pub fn from_position(target: Vector3, pos: Vector3) -> Self {
        let (yaw, pitch, r) = cart_to_sph(pos, target);
        Self { yaw, pitch, radius: r, target, spin: false, min_radius: 1.0 }
    }

    pub fn position(&self) -> Vector3 {
        sph_to_cart(self.yaw, self.pitch, self.radius, self.target)
    }

    pub fn reset(&mut self, pos: Vector3) {
        let (yaw, pitch, r) = cart_to_sph(pos, self.target);
        self.yaw = yaw; self.pitch = pitch; self.radius = r;
    }

    /// J/L: yaw, I/K: pitch, U/O: radio, P: spin toggle, T: reset
    pub fn update_input(&mut self, rl: &RaylibHandle, dt: f32) {
        let yaw_speed   = 1.5_f32;
        let pitch_speed = 1.2_f32;
        let r_speed     = 2.0_f32;

        if rl.is_key_down(KEY_J) { self.yaw   -= yaw_speed * dt; }
        if rl.is_key_down(KEY_L) { self.yaw   += yaw_speed * dt; }
        if rl.is_key_down(KEY_I) { self.pitch += pitch_speed * dt; }
        if rl.is_key_down(KEY_K) { self.pitch -= pitch_speed * dt; }
        if rl.is_key_down(KEY_U) { self.radius -= r_speed * dt; }
        if rl.is_key_down(KEY_O) { self.radius += r_speed * dt; }

        if rl.is_key_pressed(KEY_P) { self.spin = !self.spin; }
        if rl.is_key_pressed(KEY_T) {
            self.reset(self.target + Vector3::new(3.0, 4.0, 2.0));
        }
        if self.spin { self.yaw += 0.6 * dt; }

        // límites
        self.pitch  = self.pitch.clamp(-1.2, 1.2);
        self.radius = self.radius.clamp(self.min_radius, 50.0); // ← usa min_radius
    }
}

// -------- util: esféricas <-> cartesianas --------
fn sph_to_cart(yaw: f32, pitch: f32, r: f32, target: Vector3) -> Vector3 {
    let x = r * pitch.cos() * yaw.cos();
    let y = r * pitch.sin();
    let z = r * pitch.cos() * yaw.sin();
    Vector3::new(x, y, z) + target
}

fn cart_to_sph(pos: Vector3, target: Vector3) -> (f32, f32, f32) {
    let v = pos - target;
    let r = v.length();
    let yaw   = v.z.atan2(v.x);
    let pitch = (v.y / r).asin();
    (yaw, pitch, r)
}
