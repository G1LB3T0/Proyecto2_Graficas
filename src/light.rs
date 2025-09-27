use raylib::core::math::Vector3;
use raylib::RaylibHandle;
use raylib::consts::KeyboardKey::*;

use crate::util::math::{sph_to_cart, cart_to_sph};

pub struct LightRig {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub target: Vector3,
    pub spin: bool,
}

impl LightRig {
    pub fn from_position(target: Vector3, pos: Vector3) -> Self {
        let (yaw, pitch, r) = cart_to_sph(pos, target);
        Self { yaw, pitch, radius: r, target, spin: false }
    }

    pub fn position(&self) -> Vector3 {
        sph_to_cart(self.yaw, self.pitch, self.radius, self.target)
    }

    pub fn reset(&mut self, pos: Vector3) {
        let (yaw, pitch, r) = cart_to_sph(pos, self.target);
        self.yaw = yaw; self.pitch = pitch; self.radius = r;
    }

    pub fn update_input(&mut self, rl: &RaylibHandle, dt: f32) {
        let yaw_speed   = 1.5_f32; // rad/s
        let pitch_speed = 1.2_f32; // rad/s
        let r_speed     = 2.0_f32; // unidades/s

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

        if self.spin {
            self.yaw += 0.6 * dt;
        }

        // Límites razonables
        self.pitch = self.pitch.clamp(-1.2, 1.2);
        self.radius = self.radius.clamp(1.0, 12.0);
    }
}
