use raylib::core::math::Vector3;
use image::RgbaImage;

use crate::camera::OrbitCamRT;
use crate::world::{Block, Materials};

/// Modo de reflexión para agua
#[derive(Clone, Copy, Debug)]
pub enum WaterMode {
    Off,         // sin reflejo (más rápido)
    SkyOnly,     // refleja solo cielo (muy rápido, default)
    ReflectOnce, // 1 rebote de reflexión (más bonito, más lento)
}

/// Escena principal
#[derive(Clone)]
pub struct SceneRT {
    pub cam: OrbitCamRT,
    pub light_pos: Vector3,
    pub floor_color: Vector3,   // lineal 0..1
    pub show_floor: bool,
    pub blocks: Vec<Block>,
    pub mats: Materials,
    pub water_mode: WaterMode,
    pub is_night: bool,         // nuevo: modo día/noche
}

pub mod color;
mod cam;
mod sample;
mod shade;
mod fog;
mod renderer;

pub use renderer::{render, render_mt};
