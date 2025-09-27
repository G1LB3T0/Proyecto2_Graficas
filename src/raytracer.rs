use std::thread;

use raylib::core::math::Vector3;
use image::{RgbaImage, Rgba};

use crate::scene::SceneRT;
use crate::geometry::{Ray, Hit, hit_plane_y0, hit_aabb};
use crate::util::math::{clamp01, gamma_encode};

#[derive(Clone, Copy)]
struct CamPre {
    eye: Vector3,
    fwd: Vector3,
    right: Vector3,
    up: Vector3,
    aspect: f32,
    tan_half: f32,
}

fn cam_precompute(cam: &crate::camera::OrbitCamRT) -> CamPre {
    let eye = cam.eye();
    let fwd = (cam.target - eye).normalized();
    let right = fwd.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
    let up = right.cross(fwd).normalized();
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

#[inline]
fn sample_texture(tex: &RgbaImage, uv: [f32; 2]) -> Vector3 {
    let u = uv[0] - uv[0].floor(); // wrap
    let v = uv[1] - uv[1].floor();
    let x = (u * (tex.width()  as f32 - 1.0)).round() as u32;
    let y = ((1.0 - v) * (tex.height() as f32 - 1.0)).round() as u32;
    let p = tex.get_pixel(x, y);
    Vector3::new(p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0)
}

#[inline]
fn shade_pre(scene: &SceneRT, pre: &CamPre, hit: &Hit) -> Vector3 {
    let base = if hit.id == 1 { sample_texture(&scene.tex, hit.uv) } else { scene.floor_color };

    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();
    let v = (pre.eye - hit.p).normalized();
    let h = (l + v).normalized();

    let diff = clamp01(n.dot(l));
    let spec = clamp01(n.dot(h)).powf(32.0);

    // Shadow ray (sombra dura)
    let shadow = {
        let eps = 1e-3;
        let to_light = scene.light_pos - hit.p;
        let dist_l = to_light.length();
        let ray = Ray { o: hit.p + n * eps, d: to_light / dist_l };
        if let Some(hc) = hit_aabb(ray, scene.cube_center, scene.cube_half) {
            hc.t < dist_l
        } else { false }
    };

    let ambient = 0.2;
    let mut color = base * ambient;
    if !shadow {
        color += base * diff + Vector3::new(1.0, 1.0, 1.0) * 0.4 * spec;
    }
    gamma_encode(color)
}

// --------- Render single-thread (referencia) ----------
pub fn render(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = cam_precompute(&scene.cam);
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let dir = primary_dir(&pre, x, y, w, h);
            let ray = Ray { o: pre.eye, d: dir };
            let mut best = Hit::none();
            if let Some(hp) = hit_plane_y0(ray) { if hp.t < best.t { best = hp; } }
            if let Some(hc) = hit_aabb(ray, scene.cube_center, scene.cube_half) { if hc.t < best.t { best = hc; } }
            let col = if best.id >= 0 { shade_pre(scene, &pre, &best) }
                      else { Vector3::new(0.05, 0.07, 0.1) };
            let r=(clamp01(col.x)*255.0) as u8; let g=(clamp01(col.y)*255.0) as u8; let b=(clamp01(col.z)*255.0) as u8;
            img.put_pixel(x, y, Rgba([r,g,b,255]));
        }
    }
    img
}

// --------- Render multihilo (rápido) ----------
pub fn render_mt(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = cam_precompute(&scene.cam);

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
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
                    if let Some(hp) = hit_plane_y0(ray) { if hp.t < best.t { best = hp; } }
                    if let Some(hc) = hit_aabb(ray, sc.cube_center, sc.cube_half) { if hc.t < best.t { best = hc; } }
                    let col = if best.id >= 0 { shade_pre(&sc, &pre, &best) }
                              else { Vector3::new(0.05, 0.07, 0.1) };
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
