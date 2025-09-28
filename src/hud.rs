use raylib::prelude::*;

/// Entrada de texto para el HUD
struct HudEntry {
    text: String,
    color: Color,
    size: i32,
}

/// HUD simple acumulable con toggle (H)
pub struct Hud {
    pub visible: bool,
    pos: Vector2,
    line_gap: i32,
    entries: Vec<HudEntry>,
}

impl Hud {
    pub fn new() -> Self {
        Self {
            visible: true,
            pos: Vector2::new(12.0, 12.0),
            line_gap: 2,
            entries: Vec::with_capacity(16),
        }
    }

    /// Maneja el toggle de visibilidad con la tecla H
    pub fn update_input(&mut self, rl: &RaylibHandle) {
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            self.visible = !self.visible;
        }
    }

    /// Limpia las líneas para este frame
    pub fn begin_frame(&mut self) {
        self.entries.clear();
    }

    /// Añade una línea con estilo por defecto
    pub fn line<S: Into<String>>(&mut self, s: S) {
        self.line_col_size(s, Color::LIGHTGRAY, 18);
    }

    /// Añade una línea con color y tamaño
    pub fn line_col_size<S: Into<String>>(&mut self, s: S, color: Color, size: i32) {
        self.entries.push(HudEntry {
            text: s.into(),
            color,
            size,
        });
    }

    /// Dibuja el HUD (si está visible)
    pub fn draw(&self, d: &mut RaylibDrawHandle) {
        if !self.visible { return; }

        let mut y = self.pos.y as i32;

        // Sombra suave para lectura
        for e in &self.entries {
            // sombra
            d.draw_text(&e.text, self.pos.x as i32 + 1, y + 1, e.size, Color::BLACK);
            // texto
            d.draw_text(&e.text, self.pos.x as i32, y, e.size, e.color);
            y += e.size + self.line_gap;
        }
    }
}
