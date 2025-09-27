use raylib::core::math::Vector3;
use image::RgbaImage;

use crate::world::{Materials, BlockKind};
use super::color::srgb_to_linear;

#[inline]
pub fn sample_texture_linear_alpha(tex: &RgbaImage, uv: [f32; 2]) -> (Vector3, f32) {
    let u = uv[0] - uv[0].floor();
    let v = uv[1] - uv[1].floor();
    let x = (u * (tex.width()  as f32 - 1.0)).round() as u32;
    let y = ((1.0 - v) * (tex.height() as f32 - 1.0)).round() as u32;
    let p = tex.get_pixel(x, y);
    let srgb = Vector3::new(p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
    let a = p[3] as f32 / 255.0;
    (srgb_to_linear(srgb), a)
}

#[inline]
pub fn sample_block_linear_alpha(
    mats: &Materials, uv: [f32; 2], face: u8, kind: BlockKind
) -> (Vector3, f32) {
    match kind {
        BlockKind::Grass => {
            if face == 3      { sample_texture_linear_alpha(&mats.grass_top,  uv) }   // +Y
            else if face == 2 { sample_texture_linear_alpha(&mats.dirt,       uv) }   // -Y
            else              { sample_texture_linear_alpha(&mats.grass_side, uv) }   // lados
        }
        BlockKind::Dirt   => sample_texture_linear_alpha(&mats.dirt,      uv),
        BlockKind::Stone  => sample_texture_linear_alpha(&mats.stone,     uv),
        BlockKind::Log    => if face == 2 || face == 3 {
                                sample_texture_linear_alpha(&mats.log_top,  uv)
                             } else {
                                sample_texture_linear_alpha(&mats.log_side, uv)
                             },
        BlockKind::Leaves => sample_texture_linear_alpha(&mats.leaves,    uv), // alpha
        BlockKind::Water  => sample_texture_linear_alpha(&mats.water,     uv), // alpha
    }
}
