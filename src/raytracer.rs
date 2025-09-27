use std::thread;
use raylib::core::math::Vector3;
use image::{RgbaImage, Rgba};

use crate::camera::OrbitCamRT;
use crate::geometry::{Ray, Hit, hit_plane_y0, hit_aabb};
use crate::world::{Block, BlockKind, Materials};

/// Escena completa
#[derive(Clone)]
pub struct SceneRT {
    pub cam: OrbitCamRT,
    pub light_pos: Vector3,
    pub floor_color: Vector3,   // lineal 0..1
    pub blocks: Vec<Block>,
    pub mats: Materials,
}

// ===================== [ color utils ] =====================
#[inline] fn clamp01(x: f32) -> f32 { if x < 0.0 { 0.0 } else if x > 1.0 { 1.0 } else { x } }

#[inline]
fn srgb_to_linear(c: Vector3) -> Vector3 {
    Vector3::new(c.x.powf(2.2), c.y.powf(2.2), c.z.powf(2.2))
}

#[inline]
fn gamma_encode(c: Vector3) -> Vector3 {
    Vector3::new(c.x.powf(1.0/2.2), c.y.powf(1.0/2.2), c.z.powf(1.0/2.2))
}

#[inline]
fn sky_bg() -> Vector3 { Vector3::new(0.05, 0.07, 0.1) } // “cielo” simple

// ===================== [ cámara precómputo ] =====================
#[derive(Clone, Copy)]
struct CamPre {
    eye: Vector3,
    fwd: Vector3,
    right: Vector3,
    up: Vector3,
    aspect: f32,
    tan_half: f32,
}

fn cam_precompute(cam: &OrbitCamRT) -> CamPre {
    let eye   = cam.eye();
    let fwd   = (cam.target - eye).normalized();
    let right = fwd.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
    let up    = right.cross(fwd).normalized();
    let tan_half = (cam.fovy.to_radians() * 0.5).tan();
    CamPre { eye, fwd, right, up, aspect: cam.aspect, tan_half }
}

#[inline]
fn primary_dir(pre: &CamPre, x: u32, y: u32, w: u32, h: u32) -> Vector3 {
    let ndc_x = (x as f32 + 0.5) / w as f32;
    let ndc_y = (y as f32 + 0.5) / h as f32;
    let px = (2.0 * ndc_x - 1.0) * pre.aspect * pre.tan_half;
    let py = (1.0 - 2.0 * ndc_y) * pre.tan_half;
    (pre.fwd + pre.right.scale_by(px) + pre.up.scale_by(py)).normalized()
}

// ===================== [ texturas y alpha ] =====================
#[inline]
fn sample_texture_linear_alpha(tex: &RgbaImage, uv: [f32; 2]) -> (Vector3, f32) {
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
fn sample_block_linear_alpha(mats: &Materials, uv: [f32; 2], face: u8, kind: BlockKind) -> (Vector3, f32) {
    match kind {
        BlockKind::Grass => {
            if face == 3 { sample_texture_linear_alpha(&mats.grass_top, uv) }      // +Y
            else if face == 2 { sample_texture_linear_alpha(&mats.dirt, uv) }      // -Y
            else { sample_texture_linear_alpha(&mats.grass_side, uv) }             // lados
        }
        BlockKind::Dirt   => sample_texture_linear_alpha(&mats.dirt,      uv),
        BlockKind::Stone  => sample_texture_linear_alpha(&mats.stone,     uv),
        BlockKind::Log    => if face == 2 || face == 3 {
                                sample_texture_linear_alpha(&mats.log_top,  uv)
                             } else { sample_texture_linear_alpha(&mats.log_side, uv) },
        BlockKind::Leaves => sample_texture_linear_alpha(&mats.leaves,    uv),     // tiene alpha
        BlockKind::Water  => sample_texture_linear_alpha(&mats.water,     uv),     // tiene alpha
    }
}

// ===================== [ sombreado ] =====================
#[inline]
fn shade_block(pre: &CamPre, scene: &SceneRT, hit: &Hit, kind: BlockKind) -> Vector3 {
    let (base_lin, alpha) = sample_block_linear_alpha(&scene.mats, hit.uv, hit.face, kind);

    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();
    let v = (pre.eye - hit.p).normalized();
    let h = (l + v).normalized();

    let diff = clamp01(n.dot(l));
    let spec = clamp01(n.dot(h)).powf(24.0);

    // Sombras: oclusión por otros bloques (leaves no bloquea si el texel es hueco)
    let in_shadow = shadow_query(scene, hit.p, n, scene.light_pos);

    let ambient = 0.08;
    let spec_strength = 0.25;

    let mut c = base_lin * ambient;
    if !in_shadow {
        c += base_lin * diff + Vector3::new(1.0, 1.0, 1.0) * (spec_strength * spec);
    }

    // Transparencias simples: leaves/agua mezclan con "cielo"
    match kind {
        BlockKind::Leaves | BlockKind::Water => {
            let bg = sky_bg();
            let a = alpha.clamp(0.0, 1.0);
            c = bg * (1.0 - a) + c * a;
        }
        _ => {}
    }

    gamma_encode(c)
}

// Para piso (id=0) y fondo
#[inline]
fn shade_floor(pre: &CamPre, scene: &SceneRT, hit: &Hit) -> Vector3 {
    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();
    let v = (pre.eye - hit.p).normalized();
    let h = (l + v).normalized();

    let diff = clamp01(n.dot(l));
    let spec = clamp01(n.dot(h)).powf(24.0);
    let ambient = 0.06;

    let in_shadow = shadow_query(scene, hit.p, n, scene.light_pos);

    let mut c = scene.floor_color * ambient;
    if !in_shadow {
        c += scene.floor_color * diff + Vector3::new(1.0, 1.0, 1.0) * 0.15 * spec;
    }
    gamma_encode(c)
}

// Consulta de sombras contra TODOS los bloques (alpha-cutout para leaves)
fn shadow_query(scene: &SceneRT, p: Vector3, n: Vector3, light_pos: Vector3) -> bool {
    let eps = 1e-3;
    let to_light = light_pos - p;
    let dist_l = to_light.length();
    let ray = Ray { o: p + n * eps, d: to_light / dist_l };

    for b in &scene.blocks {
        if let Some(h) = hit_aabb(ray, b.center, b.half) {
            if h.t < dist_l {
                // leaves no bloquea si el texel es agujero; water no bloquea (simple)
                match b.kind {
                    BlockKind::Leaves => {
                        let (_c, a) = sample_block_linear_alpha(&scene.mats, h.uv, h.face, b.kind);
                        if a >= 0.1 { return true; }
                    }
                    BlockKind::Water => { /* puedes decidir que sí/ no bloquee; de momento no */ }
                    _ => return true,
                }
            }
        }
    }
    false
}

// ===================== [ renderers ] =====================
pub fn render(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = cam_precompute(&scene.cam);
    let mut img = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let dir = primary_dir(&pre, x, y, w, h);
            let ray = Ray { o: pre.eye, d: dir };

            let mut best = Hit::none();
            let mut best_kind: Option<BlockKind> = None;

            if let Some(hp) = hit_plane_y0(ray) { if hp.t < best.t { best = hp; best_kind = None; } }

            for b in &scene.blocks {
                if let Some(hc) = hit_aabb(ray, b.center, b.half) {
                    // Cutout de leaves: si el texel es "agujero", ignora este hit
                    if matches!(b.kind, BlockKind::Leaves) {
                        let (_c, a) = sample_block_linear_alpha(&scene.mats, hc.uv, hc.face, b.kind);
                        if a < 0.1 { continue; }
                    }
                    if hc.t < best.t {
                        best = hc; best_kind = Some(b.kind);
                    }
                }
            }

            let col = if best.id >= 0 {
                if let Some(k) = best_kind { shade_block(&pre, scene, &best, k) }
                else { shade_floor(&pre, scene, &best) }
            } else {
                sky_bg() // fondo
            };

            let r=(clamp01(col.x)*255.0) as u8; let g=(clamp01(col.y)*255.0) as u8; let b=(clamp01(col.z)*255.0) as u8;
            img.put_pixel(x, y, Rgba([r,g,b,255]));
        }
    }
    img
}

pub fn render_mt(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = cam_precompute(&scene.cam);

    let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let threads = threads.min(h as usize);
    let rows_per = ((h as usize) + threads - 1) / threads;

    let mut handles = Vec::with_capacity(threads);

    for t in 0..threads {
        let y0 = (t * rows_per) as u32;
        if y0 >= h { break; }
        let y1 = ((t + 1) * rows_per).min(h as usize) as u32;

        let sc = scene.clone();
        handles.push(thread::spawn(move || {
            let mut strip = RgbaImage::new(w, y1 - y0);
            for y in y0..y1 {
                for x in 0..w {
                    let dir = primary_dir(&pre, x, y, w, h);
                    let ray = Ray { o: pre.eye, d: dir };

                    let mut best = Hit::none();
                    let mut best_kind: Option<BlockKind> = None;

                    if let Some(hp) = hit_plane_y0(ray) { if hp.t < best.t { best = hp; best_kind = None; } }
                    for b in &sc.blocks {
                        if let Some(hc) = hit_aabb(ray, b.center, b.half) {
                            if matches!(b.kind, BlockKind::Leaves) {
                                let (_c, a) = sample_block_linear_alpha(&sc.mats, hc.uv, hc.face, b.kind);
                                if a < 0.1 { continue; }
                            }
                            if hc.t < best.t { best = hc; best_kind = Some(b.kind); }
                        }
                    }

                    let col = if best.id >= 0 {
                        if let Some(k) = best_kind { shade_block(&pre, &sc, &best, k) }
                        else { shade_floor(&pre, &sc, &best) }
                    } else {
                        sky_bg()
                    };

                    let r=(clamp01(col.x)*255.0) as u8; let g=(clamp01(col.y)*255.0) as u8; let b=(clamp01(col.z)*255.0) as u8;
                    strip.put_pixel(x, y - y0, Rgba([r,g,b,255]));
                }
            }
            (y0, strip)
        }));
    }

    let mut img = RgbaImage::new(w, h);
    for hnd in handles {
        let (y0, strip) = hnd.join().unwrap();
        for (x, y, p) in strip.enumerate_pixels() {
            img.put_pixel(x, y + y0, *p);
        }
    }
    img
}
