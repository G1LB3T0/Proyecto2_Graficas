use std::thread;
use std::sync::Arc;
use image::{RgbaImage, Rgba};
use raylib::core::math::Vector3;

use crate::geometry::{Hit};
use crate::world::BlockKind;
use super::SceneRT;

use super::cam::{precompute, primary_dir};
use super::shade::{shade_block, shade_floor};
use super::sample::sample_block_linear_alpha;
use super::fog::sky_srgb;
use super::color::clamp01;

// ====== Voxel Grid cache (derivado de scene.blocks) ======
#[derive(Clone)]
struct Grid {
    w: i32, h: i32, d: i32,
    min: Vector3,                // esquina mínima (borde), tamaño celda=1
    data: Vec<u8>,               // 0=vacío; 1..=tipo (k+1)
}
#[inline] fn kind_to_u8(k: BlockKind) -> u8 {
    match k {
        BlockKind::Grass => 1,
        BlockKind::Dirt  => 2,
        BlockKind::Stone => 3,
        BlockKind::Log   => 4,
        BlockKind::Leaves=> 5,
        BlockKind::Water => 6,
    }
}
#[inline] fn u8_to_kind(v: u8) -> Option<BlockKind> {
    match v {
        1 => Some(BlockKind::Grass),
        2 => Some(BlockKind::Dirt),
        3 => Some(BlockKind::Stone),
        4 => Some(BlockKind::Log),
        5 => Some(BlockKind::Leaves),
        6 => Some(BlockKind::Water),
        _ => None,
    }
}
#[inline] fn gidx(g:&Grid, x:i32,y:i32,z:i32) -> usize {
    (y as usize)* (g.w as usize)*(g.d as usize) + (z as usize)*(g.w as usize) + (x as usize)
}
fn build_grid(scene:&SceneRT) -> Grid {
    // X/Z están centrados alrededor de 0 y YA son enteros en centros. Y puede tener offset.
    // Hallar rangos Y enteros a partir de blocks:
    let mut miny = f32::INFINITY;
    let mut maxy = -f32::INFINITY;
    for b in &scene.blocks {
        miny = miny.min(b.center.y);
        maxy = maxy.max(b.center.y);
    }
    // capa inferior/ superior como centros
    let y0 = (miny - 0.5).floor() + 0.5;
    let y1 = (maxy + 0.5).ceil() - 0.5;
    let h = (y1 - y0 + 1.0).max(1.0) as i32;

    let w = 16i32;
    let d = 16i32;
    let min = Vector3::new(-(w as f32)*0.5, y0-0.5, -(d as f32)*0.5);
    let mut data = vec![0u8; (w*d*h) as usize];

    // mapear blocks -> celdas (solo superficie; perfecto para primario+sombra)
    for b in &scene.blocks {
        let ix = ((b.center.x + (w as f32)*0.5).floor() as i32).clamp(0, w-1);
        let iy = ((b.center.y - y0).round() as i32).clamp(0, h-1);
        let iz = ((b.center.z + (d as f32)*0.5).floor() as i32).clamp(0, d-1);
        let k = kind_to_u8(b.kind);
        data[gidx(&Grid{w,h,d,min,data:Vec::new()}, ix,iy,iz)] = k;
    }

    Grid{ w,h,d, min, data }
}

// ====== Ray vs AABB (grid global) ======
#[inline]
fn ray_aabb(o:Vector3, d:Vector3, mn:Vector3, mx:Vector3) -> Option<(f32,f32)> {
    let inv = Vector3::new(1.0/d.x, 1.0/d.y, 1.0/d.z);
    let mut t0 = (mn - o) * inv;
    let mut t1 = (mx - o) * inv;
    if t0.x > t1.x { std::mem::swap(&mut t0.x, &mut t1.x); }
    if t0.y > t1.y { std::mem::swap(&mut t0.y, &mut t1.y); }
    if t0.z > t1.z { std::mem::swap(&mut t0.z, &mut t1.z); }
    let tmin = t0.x.max(t0.y.max(t0.z));
    let tmax = t1.x.min(t1.y.min(t1.z));
    if tmax < tmin || tmax <= 1e-4 { None } else { Some((tmin.max(1e-4), tmax)) }
}

// ====== DDA traversal: primer hit en la rejilla ======
struct DdaHit {
    t: f32, p: Vector3, face: u8, n: Vector3, uv: [f32;2], kind: BlockKind
}

fn trace_grid_first(o:Vector3, d:Vector3, g:&Grid, mats:&crate::world::Materials) -> Option<DdaHit> {
    let max = g.min + Vector3::new(g.w as f32, g.h as f32, g.d as f32);
    let (mut t, tmax_all) = ray_aabb(o,d,g.min,max)?;
    // punto de entrada
    let mut p = o + d*t;

    // índices de celda
    let mut ix = ((p.x - g.min.x).floor() as i32).clamp(0, g.w-1);
    let mut iy = ((p.y - g.min.y).floor() as i32).clamp(0, g.h-1);
    let mut iz = ((p.z - g.min.z).floor() as i32).clamp(0, g.d-1);

    // pasos y t next
    let (stepx, stepy, stepz) = (
        if d.x>0.0 {1} else {-1},
        if d.y>0.0 {1} else {-1},
        if d.z>0.0 {1} else {-1},
    );

    let mut next_x = g.min.x + (if d.x>0.0 { (ix+1) as f32 } else { ix as f32 });
    let mut next_y = g.min.y + (if d.y>0.0 { (iy+1) as f32 } else { iy as f32 });
    let mut next_z = g.min.z + (if d.z>0.0 { (iz+1) as f32 } else { iz as f32 });

    let mut tmaxx = if d.x!=0.0 {(next_x - o.x)/d.x} else { f32::INFINITY };
    let mut tmaxy = if d.y!=0.0 {(next_y - o.y)/d.y} else { f32::INFINITY };
    let mut tmaxz = if d.z!=0.0 {(next_z - o.z)/d.z} else { f32::INFINITY };

    let tdx = if d.x!=0.0 { (1.0/d.x).abs() } else { f32::INFINITY };
    let tdy = if d.y!=0.0 { (1.0/d.y).abs() } else { f32::INFINITY };
    let tdz = if d.z!=0.0 { (1.0/d.z).abs() } else { f32::INFINITY };

    let mut face:u8 = 255;

    while t <= tmax_all {
        // ¿ocupada?
        if ix>=0 && ix<g.w && iy>=0 && iy<g.h && iz>=0 && iz<g.d {
            let v = g.data[gidx(g, ix,iy,iz)];
            if let Some(kind) = u8_to_kind(v) {
                // calcular normal/UV a partir de la cara de entrada (face)
                let n = match face {
                    0 => Vector3::new(-1.0,0.0,0.0),
                    1 => Vector3::new( 1.0,0.0,0.0),
                    2 => Vector3::new(0.0,-1.0,0.0),
                    3 => Vector3::new(0.0, 1.0,0.0),
                    4 => Vector3::new(0.0,0.0,-1.0),
                    5 => Vector3::new(0.0,0.0, 1.0),
                    _ => {
                        // primer voxel (t==tmin), escoger cara por min de tmax*
                        if tmaxx <= tmaxy && tmaxx <= tmaxz {
                            if stepx==1 { Vector3::new(-1.0,0.0,0.0) } else { Vector3::new(1.0,0.0,0.0) }
                        } else if tmaxy <= tmaxz {
                            if stepy==1 { Vector3::new(0.0,-1.0,0.0) } else { Vector3::new(0.0,1.0,0.0) }
                        } else {
                            if stepz==1 { Vector3::new(0.0,0.0,-1.0) } else { Vector3::new(0.0,0.0,1.0) }
                        }
                    }
                };

                // punto de impacto
                p = o + d*t;

                // centro del voxel
                let cx = g.min.x + ix as f32 + 0.5;
                let cy = g.min.y + iy as f32 + 0.5;
                let cz = g.min.z + iz as f32 + 0.5;

                // UV por cara (igual que en AABB)
                let size = 1.0;
                let s = Vector3::new(
                    (p.x - (cx-0.5))/size,
                    (p.y - (cy-0.5))/size,
                    (p.z - (cz-0.5))/size
                );
                let f = face_for_normal(n);
                let uv = match f {
                    // LADOS: v = s.y (↑)
                    0 => [1.0 - s.z, s.y], // -X
                    1 => [s.z,       s.y], // +X
                    // TOP/BOTTOM (igual que antes)
                    2 => [s.x,       s.z      ], // -Y
                    3 => [s.x,       1.0 - s.z], // +Y
                    // LADOS Z: v = s.y (↑)
                    4 => [s.x,       s.y],      // -Z
                    5 => [1.0 - s.x, s.y],      // +Z
                    _ => [0.0,0.0]
                };

                // Cutout de hojas: si alpha baja, sigue el DDA (no es hit sólido)
                if let BlockKind::Leaves = kind {
                    let (_c, a) = sample_block_linear_alpha(mats, uv, f, kind);
                    if a < 0.1 { /* pasa luz/visión */ }
                    else {
                        return Some(DdaHit{ t, p, face:f, n, uv, kind });
                    }
                } else {
                    return Some(DdaHit{ t, p, face:f, n, uv, kind });
                }
            }
        }

        // avanzar a siguiente plano
        if tmaxx <= tmaxy && tmaxx <= tmaxz {
            t = tmaxx; tmaxx += tdx; ix += stepx; face = if stepx==1 {0} else {1};
        } else if tmaxy <= tmaxz {
            t = tmaxy; tmaxy += tdy; iy += stepy; face = if stepy==1 {2} else {3};
        } else {
            t = tmaxz; tmaxz += tdz; iz += stepz; face = if stepz==1 {4} else {5};
        }
    }
    None
}

#[inline]
fn face_for_normal(n: Vector3) -> u8 {
    if n.x < -0.5 { 0 } else if n.x > 0.5 { 1 }
    else if n.y < -0.5 { 2 } else if n.y > 0.5 { 3 }
    else if n.z < -0.5 { 4 } else { 5 }
}

// ====== RENDERERS =================================================
pub fn render(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = precompute(&scene.cam);
    let grid = build_grid(scene);
    let mut img = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let dir = primary_dir(&pre, x, y, w, h);

            let col = if let Some(hh) = trace_grid_first(pre.eye, dir, &grid, &scene.mats) {
                // Adaptar a Hit y sombrear
                let hit = Hit { id:1, t:hh.t, p:hh.p, n:hh.n, uv:hh.uv, face:hh.face };
                shade_block(&pre, scene, &hit, hh.kind)
            } else if scene.show_floor {
                // opcional: piso; por simplicidad lo omitimos del DDA si show_floor=false
                let bg = sky_srgb(dir);
                bg
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

pub fn render_mt(scene: &SceneRT, w: u32, h: u32) -> RgbaImage {
    let pre = precompute(&scene.cam);
    let grid = Arc::new(build_grid(scene));

    let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let threads = threads.min(h as usize);
    let rows_per = ((h as usize) + threads - 1) / threads;

    let mut handles = Vec::with_capacity(threads);

    for t in 0..threads {
        let y0 = (t * rows_per) as u32;
        if y0 >= h { break; }
        let y1 = ((t + 1) * rows_per).min(h as usize) as u32;

        let sc = scene.clone();
        let g = grid.clone();
        handles.push(thread::spawn(move || {
            let mut strip = RgbaImage::new(w, y1 - y0);
            for y in y0..y1 {
                for x in 0..w {
                    let dir = super::cam::primary_dir(&pre, x, y, w, h);

                    let col = if let Some(hh) = trace_grid_first(pre.eye, dir, &g, &sc.mats) {
                        let hit = Hit { id:1, t:hh.t, p:hh.p, n:hh.n, uv:hh.uv, face:hh.face };
                        super::shade::shade_block(&pre, &sc, &hit, hh.kind)
                    } else if sc.show_floor {
                        sky_srgb(dir)
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
