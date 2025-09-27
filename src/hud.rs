use raylib::prelude::*;
use raylib::consts::KeyboardKey::*;

pub struct Hud {
    pub show: bool,
}

impl Hud {
    pub fn new() -> Self { Self { show: true } }

    /// H: alterna mostrar/ocultar HUD
    pub fn update_input(&mut self, rl: &RaylibHandle) {
        if rl.is_key_pressed(KEY_H) { self.show = !self.show; }
    }

    /// Dibuja tips y FPS
    pub fn draw(&self, d: &mut RaylibDrawHandle) {
        if !self.show { return; }

        let x = 10;
        let mut y = 10;
        let lh = 18;

        d.draw_text("CPU RT | Mouse L: orbit | Wheel: zoom | R: reset | F1: res 1/2<->1x | H: HUD",
                    x, y, lh, Color::RED);
        y += 22;
        d.draw_text("Light: J/L yaw | I/K pitch | U/O radius | P spin | T reset",
                    x, y, lh, Color::RED);
        y += 22;

        d.draw_fps(x, y + 10);
    }
}
