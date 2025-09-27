use raylib::core::math::Vector3;
use image::RgbaImage;

use crate::camera::OrbitCamRT;

#[derive(Clone)]
pub struct SceneRT {
    pub cam: OrbitCamRT,
    pub light_pos: Vector3,
    pub floor_color: Vector3,
    pub cube_center: Vector3,
    pub cube_half: f32,
    pub tex: RgbaImage,
}
