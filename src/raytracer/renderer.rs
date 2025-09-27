use std::thread;
use image::{RgbaImage, Rgba};

use crate::geometry::{Hit, hit_plane_y0, hit_aabb};
use crate::world::BlockKind;
use super::SceneRT;

use super::cam::{precompute, primary_dir};
use super::shade::{shade_block, shade_floor};
use super::sample::sample_block_linear_alpha;
use super::fog::sky_srgb;
use super::color::clamp01;

// ---------------- single-thread ----------------
pub fn render(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = precompute(&scene.cam);
    let mut img = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let dir = primary_dir(&pre, x, y, w, h);

            let mut best = Hit::none();
            let mut best_kind: Option<BlockKind> = None;

            // piso opcional
            if scene.show_floor {
                if let Some(hp) = hit_plane_y0(crate::geometry::Ray { o: pre.eye, d: dir }) {
                    if hp.t < best.t { best = hp; best_kind = None; }
                }
            }

            // bloques
            for b in &scene.blocks {
                if let Some(hc) = hit_aabb(crate::geometry::Ray { o: pre.eye, d: dir }, b.center, b.half) {
                    // leaves cutout
                    if matches!(b.kind, BlockKind::Leaves) {
                        let (_c, a) = sample_block_linear_alpha(&scene.mats, hc.uv, hc.face, b.kind);
                        if a < 0.1 { continue; }
                    }
                    if hc.t < best.t { best = hc; best_kind = Some(b.kind); }
                }
            }

            // color final (sin fog) + cielo de fondo
            let col = if best.id >= 0 {
                match best_kind {
                    Some(k) => shade_block(&pre, scene, &best, k),
                    None if scene.show_floor => shade_floor(&pre, scene, &best),
                    _ => sky_srgb(dir),
                }
            } else {
                sky_srgb(dir)
            };

            let r=(clamp01(col.x)*255.0) as u8;
            let g=(clamp01(col.y)*255.0) as u8;
            let b=(clamp01(col.z)*255.0) as u8;
            img.put_pixel(x, y, Rgba([r,g,b,255]));
        }
    }
    img
}

// ---------------- multi-thread ----------------
pub fn render_mt(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = precompute(&scene.cam);

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

                    let mut best = Hit::none();
                    let mut best_kind: Option<BlockKind> = None;

                    if sc.show_floor {
                        if let Some(hp) = hit_plane_y0(crate::geometry::Ray { o: pre.eye, d: dir }) {
                            if hp.t < best.t { best = hp; best_kind = None; }
                        }
                    }

                    for b in &sc.blocks {
                        if let Some(hc) = hit_aabb(crate::geometry::Ray { o: pre.eye, d: dir }, b.center, b.half) {
                            if matches!(b.kind, BlockKind::Leaves) {
                                let (_c, a) = sample_block_linear_alpha(&sc.mats, hc.uv, hc.face, b.kind);
                                if a < 0.1 { continue; }
                            }
                            if hc.t < best.t { best = hc; best_kind = Some(b.kind); }
                        }
                    }

                    let col = if best.id >= 0 {
                        match best_kind {
                            Some(k) => shade_block(&pre, &sc, &best, k),
                            None if sc.show_floor => shade_floor(&pre, &sc, &best),
                            _ => sky_srgb(dir),
                        }
                    } else {
                        sky_srgb(dir)
                    };

                    let r=(clamp01(col.x)*255.0) as u8;
                    let g=(clamp01(col.y)*255.0) as u8;
                    let b=(clamp01(col.z)*255.0) as u8;
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
