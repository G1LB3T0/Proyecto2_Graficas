use raylib::core::math::Vector3;
use image::RgbaImage;

use crate::camera::OrbitCamRT;
use crate::world::{Block, Materials};

/// Escena principal (misma interfaz pública de antes)
#[derive(Clone)]
pub struct SceneRT {
    pub cam: OrbitCamRT,
    pub light_pos: Vector3,
    pub floor_color: Vector3,   // lineal 0..1
    pub show_floor: bool,       // mostrar/ocultar plano Y=0 (si lo usas)
    pub blocks: Vec<Block>,
    pub mats: Materials,
}

pub mod color;
mod cam;
mod sample;
mod shade;
mod fog;
mod renderer;

pub use renderer::{render, render_mt};
