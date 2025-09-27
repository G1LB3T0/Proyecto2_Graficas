use raylib::core::math::Vector3;

use crate::geometry::{Ray, Hit, hit_aabb};
use crate::world::BlockKind;
use crate::raytracer::SceneRT;

use super::cam::CamPre;
use super::color::{clamp01, gamma_encode};
use super::sample::sample_block_linear_alpha;

#[inline]
pub fn shade_block(pre: &CamPre, scene: &SceneRT, hit: &Hit, kind: BlockKind) -> Vector3 {
    let (base_lin, _alpha) = sample_block_linear_alpha(&scene.mats, hit.uv, hit.face, kind);

    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();
    let v = (pre.eye - hit.p).normalized();
    let _h = (l + v).normalized();

    // ---- Iluminación sin especular (evita el “foco”) ----
    // Wrap Lambert para suavizar el hotspot del difuso
    let k_wrap = 0.25; // 0..0.4 aprox. (más alto = más suave)
    let diff = clamp01((n.dot(l) + k_wrap) / (1.0 + k_wrap));

    // Sombra por oclusión de otros bloques
    let in_shadow = shadow_query(scene, hit.p, n, scene.light_pos);

    // Un poco más de ambiente para que no se vea apagado
    let ambient = 0.12;

    let mut c = base_lin * ambient;
    if !in_shadow {
        c += base_lin * diff;
    }

    gamma_encode(c)
}

#[inline]
pub fn shade_floor(pre: &CamPre, scene: &SceneRT, hit: &Hit) -> Vector3 {
    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();

    // Misma lógica sin especular
    let k_wrap = 0.25;
    let diff = clamp01((n.dot(l) + k_wrap) / (1.0 + k_wrap));

    let in_shadow = shadow_query(scene, hit.p, n, scene.light_pos);

    let ambient = 0.10;
    let mut c = scene.floor_color * ambient;
    if !in_shadow {
        c += scene.floor_color * diff;
    }
    gamma_encode(c)
}

/// Sombras: intersección con otros bloques.
/// Leaves hace cutout: sólo bloquea si el texel muestreado es opaco.
pub fn shadow_query(scene: &SceneRT, p: Vector3, n: Vector3, light_pos: Vector3) -> bool {
    let eps = 1e-3;
    let to_light = light_pos - p;
    let dist_l = to_light.length();
    let ray = Ray { o: p + n * eps, d: to_light / dist_l };

    for b in &scene.blocks {
        if let Some(h) = hit_aabb(ray, b.center, b.half) {
            if h.t < dist_l {
                match b.kind {
                    BlockKind::Leaves => {
                        let (_c, a) = sample_block_linear_alpha(&scene.mats, h.uv, h.face, b.kind);
                        if a >= 0.1 { return true; }
                    }
                    BlockKind::Water => { /* no bloquea sombra */ }
                    _ => return true,
                }
            }
        }
    }
    false
}
