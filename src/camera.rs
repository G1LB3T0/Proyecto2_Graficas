use raylib::prelude::*;

#[derive(Clone, Copy)]
pub struct OrbitCamRT {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub target: Vector3,
    pub fovy: f32,
    pub aspect: f32,
}

impl OrbitCamRT {
    pub fn new(target: Vector3, aspect: f32) -> Self {
        Self {
            yaw: std::f32::consts::FRAC_PI_4,
            pitch: 0.4,
            radius: 6.0,
            target,
            fovy: 45.0,
            aspect,
        }
    }

    pub fn apply_input(&mut self, rl: &RaylibHandle) {
        use raylib::consts::{KeyboardKey::*, MouseButton::*};
        if rl.is_mouse_button_down(MOUSE_BUTTON_LEFT) {
            let md = rl.get_mouse_delta();
            self.yaw   -= md.x * 0.005;
            self.pitch += md.y * 0.005;
            self.pitch = self.pitch.clamp(-1.45, 1.45);
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel.abs() > 0.0 {
            self.radius = (self.radius - wheel * 0.6).clamp(2.0, 30.0);
        }
        if rl.is_key_pressed(KEY_R) {
            self.yaw = std::f32::consts::FRAC_PI_4;
            self.pitch = 0.4;
            self.radius = 6.0;
        }
    }

    pub fn eye(&self) -> Vector3 {
        let x = self.radius * self.pitch.cos() * self.yaw.cos();
        let y = self.radius * self.pitch.sin();
        let z = self.radius * self.pitch.cos() * self.yaw.sin();
        Vector3::new(x, y, z) + self.target
    }
}
