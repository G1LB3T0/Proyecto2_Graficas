use raylib::core::math::Vector3;
use crate::camera::OrbitCamRT;

#[derive(Clone, Copy)]
pub struct CamPre {
    pub eye: Vector3,
    pub fwd: Vector3,
    pub right: Vector3,
    pub up: Vector3,
    pub aspect: f32,
    pub tan_half: f32,
}

pub fn precompute(cam: &OrbitCamRT) -> CamPre {
    let eye   = cam.eye();
    let fwd   = (cam.target - eye).normalized();
    let right = fwd.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
    let up    = right.cross(fwd).normalized();
    let tan_half = (cam.fovy.to_radians() * 0.5).tan();
    CamPre { eye, fwd, right, up, aspect: cam.aspect, tan_half }
}

#[inline]
pub fn primary_dir(pre: &CamPre, x: u32, y: u32, w: u32, h: u32) -> Vector3 {
    let ndc_x = (x as f32 + 0.5) / w as f32;
    let ndc_y = (y as f32 + 0.5) / h as f32;
    let px = (2.0 * ndc_x - 1.0) * pre.aspect * pre.tan_half;
    let py = (1.0 - 2.0 * ndc_y) * pre.tan_half;
    (pre.fwd + pre.right.scale_by(px) + pre.up.scale_by(py)).normalized()
}
