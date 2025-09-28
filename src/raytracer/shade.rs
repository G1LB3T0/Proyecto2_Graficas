use raylib::core::math::Vector3;

use crate::geometry::{Ray, Hit, hit_aabb, hit_plane_y0};
use crate::world::BlockKind;
use crate::raytracer::{SceneRT, WaterMode};

use super::cam::CamPre;
use super::color::{clamp01, gamma_encode};
use super::sample::sample_block_linear_alpha;
use super::fog::sky_srgb;

/// Hash determinista 0..1 para dithering en sombras (hojas)
#[inline]
fn hash01(a: f32, b: f32, c: f32) -> f32 {
    let x = a * 127.1 + b * 311.7 + c * 74.7;
    let mut h = (x.sin() * 43758.5453).fract();
    if h < 0.0 { h += 1.0; }
    h
}

#[inline]
fn reflect(i: Vector3, n: Vector3) -> Vector3 {
    i - n * (2.0 * i.dot(n))
}

#[inline]
fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    // F = F0 + (1-F0)(1-cosθ)^5
    f0 + (1.0 - f0) * (1.0 - cos_theta).powf(5.0)
}

#[inline]
pub fn shade_block(pre: &CamPre, scene: &SceneRT, hit: &Hit, kind: BlockKind) -> Vector3 {
    let (base_lin, alpha) = sample_block_linear_alpha(&scene.mats, hit.uv, hit.face, kind);

    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();
    let v = (pre.eye - hit.p).normalized();

    // Difuso sin especular (evita “foco”)
    let k_wrap = 0.25;
    let diff = clamp01((n.dot(l) + k_wrap) / (1.0 + k_wrap));

    let in_shadow = shadow_query(scene, hit.p, n, scene.light_pos);

    // Ambiente un poco más alto para look suave
    let ambient = 0.12;
    let mut c_lin = base_lin * ambient;
    if !in_shadow {
        c_lin += base_lin * diff;
    }

    match kind {
        BlockKind::Leaves | BlockKind::Water => {
            // Mezcla simple con cielo según alpha (para ver “huecos” y transparencia)
            // NOTA: ahora si es Agua, además añadimos Fresnel/reflexión abajo.
            let dir_to_eye = (hit.p - pre.eye).normalized();
            let bg = sky_srgb(dir_to_eye);
            let a = alpha.clamp(0.0, 1.0);
            let mut c_srgb = gamma_encode(c_lin);
            c_srgb = bg * (1.0 - a) + c_srgb * a;

            if matches!(kind, BlockKind::Water) {
                // --- AGUA: reflexión Fresnel + modos de rendimiento ---
                // Vector de incidencia hacia la superficie:
                let i = -v;               // de cámara a superficie
                let r = reflect(i, n);    // dirección reflejada
                let cos_theta = clamp01(n.dot(v));
                let kr = fresnel_schlick(cos_theta, 0.02); // F0 ~ 0.02 para agua

                // color reflejado:
                let refl_srgb = match scene.water_mode {
                    WaterMode::Off      => Vector3::new(0.0, 0.0, 0.0),
                    WaterMode::SkyOnly  => sky_srgb(r),
                    WaterMode::ReflectOnce => trace_reflect_once(pre, scene, hit.p, r),
                };

                // composición final: mezcla Fresnel entre transmisión y reflexión
                // c_srgb ~ transmisión sRGB; refl_srgb ya en sRGB
                return c_srgb * (1.0 - kr) + refl_srgb * kr;
            }

            c_srgb
        }
        _ => gamma_encode(c_lin),
    }
}

/// Reflejo de 1 rebote: lanza un rayo y sombrea SOLO el primer hit (sin recursión).
/// Si impacta agua, devolvemos cielo (evitamos cascada de reflexiones).
fn trace_reflect_once(pre: &CamPre, scene: &SceneRT, origin: Vector3, dir: Vector3) -> Vector3 {
    let eps = 1e-3;
    let ray = Ray { o: origin + dir * eps, d: dir };

    let mut best = Hit::none();
    let mut best_kind: Option<BlockKind> = None;

    if scene.show_floor {
        if let Some(hp) = hit_plane_y0(ray) { if hp.t < best.t { best = hp; best_kind = None; } }
    }

    for b in &scene.blocks {
        if let Some(hc) = hit_aabb(ray, b.center, b.half) {
            // leaves cutout
            if matches!(b.kind, BlockKind::Leaves) {
                let (_c, a) = sample_block_linear_alpha(&scene.mats, hc.uv, hc.face, b.kind);
                if a < 0.1 { continue; }
            }
            if hc.t < best.t { best = hc; best_kind = Some(b.kind); }
        }
    }

    if best.id < 0 {
        return sky_srgb(dir);
    }

    match best_kind {
        Some(BlockKind::Water) => sky_srgb(dir), // no reflexionar el agua recursivamente
        Some(k) => {
            // Sombréalo con el mismo modelo (sin reflexiones extra)
            // OJO: shade_block puede volver a llamar shadow_query, pero no refleja.
            shade_block(pre, scene, &best, k)
        }
        None if scene.show_floor => shade_floor(pre, scene, &best),
        _ => sky_srgb(dir),
    }
}

#[inline]
pub fn shade_floor(_pre: &CamPre, scene: &SceneRT, hit: &Hit) -> Vector3 {
    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();

    let k_wrap = 0.25;
    let diff = clamp01((n.dot(l) + k_wrap) / (1.0 + k_wrap));

    let in_shadow = shadow_query(scene, hit.p, n, scene.light_pos);

    let ambient = 0.10;
    let mut c_lin = scene.floor_color * ambient;
    if !in_shadow {
        c_lin += scene.floor_color * diff;
    }
    gamma_encode(c_lin)
}

/// Sombras con “cutout dither” para hojas.
/// Agua: no bloquea (puedes cambiarlo si quieres).
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
                        let m = hash01(h.uv[0] * 64.0, h.uv[1] * 64.0, h.face as f32);
                        if a > m { return true; } else { continue; }
                    }
                    BlockKind::Water => { /* no bloquea sombra */ }
                    _ => return true,
                }
            }
        }
    }
    false
}
