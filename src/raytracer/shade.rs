use raylib::core::math::Vector3;

use crate::geometry::{Ray, Hit};
use crate::world::BlockKind;
use crate::raytracer::{SceneRT, WaterMode};

use super::cam::CamPre;
use super::color::{clamp01, gamma_encode};
use super::sample::sample_block_linear_alpha;
use super::fog::sky_srgb;

// ---- util ----
#[inline] fn hash01(a: f32, b: f32, c: f32) -> f32 {
    let x = a * 127.1 + b * 311.7 + c * 74.7;
    let mut h = (x.sin() * 43758.5453).fract();
    if h < 0.0 { h += 1.0; }
    h
}
#[inline] fn reflect(i: Vector3, n: Vector3) -> Vector3 { i - n * (2.0 * i.dot(n)) }
#[inline] fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 { f0 + (1.0 - f0) * (1.0 - cos_theta).powf(5.0) }

pub fn shade_block(pre: &CamPre, scene: &SceneRT, hit: &Hit, kind: BlockKind) -> Vector3 {
    let (base_lin, alpha) = sample_block_linear_alpha(&scene.mats, hit.uv, hit.face, kind);

    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();
    let v = (pre.eye - hit.p).normalized();

    // Difuso suave (sin especular)
    let k_wrap = 0.25;
    let diff = clamp01((n.dot(l) + k_wrap) / (1.0 + k_wrap));

    let in_shadow = shadow_query_fast(scene, hit.p, n);

    let ambient = 0.12;
    let mut c_lin = base_lin * ambient;
    if !in_shadow { c_lin += base_lin * diff; }

    match kind {
        BlockKind::Leaves | BlockKind::Water => {
            // transparencia base (look fancy)
            let dir = (hit.p - pre.eye).normalized();
            let bg = sky_srgb(dir);
            let a = alpha.clamp(0.0, 1.0);
            let mut c_srgb = gamma_encode(c_lin);
            c_srgb = bg * (1.0 - a) + c_srgb * a;

            if let BlockKind::Water = kind {
                // Fresnel
                let i = -v; let r = reflect(i, n);
                let cos_theta = clamp01(n.dot(v));
                let kr = fresnel_schlick(cos_theta, 0.02);
                let refl_srgb = match scene.water_mode {
                    WaterMode::Off => Vector3::new(0.0,0.0,0.0),
                    WaterMode::SkyOnly => sky_srgb(r),
                    WaterMode::ReflectOnce => trace_reflect_once(pre, scene, hit.p, r),
                };
                return c_srgb * (1.0 - kr) + refl_srgb * kr;
            }
            c_srgb
        }
        _ => gamma_encode(c_lin),
    }
}

fn trace_reflect_once(pre:&CamPre, scene:&SceneRT, origin:Vector3, dir:Vector3) -> Vector3 {
    // Usa el mismo DDA del renderer construyendo una grid ligera aquí.
    // Para no duplicar lógica, llamamos a una versión simplificada local.
    if let Some((hit, kind)) = first_hit_fast(scene, origin, dir) {
        match kind {
            BlockKind::Water => sky_srgb(dir),
            k => shade_block(pre, scene, &hit, k),
        }
    } else { sky_srgb(dir) }
}

// ===== Sombra y primer impacto: versión "rápida" basada en rejilla =====

// Construye grid ligero en cada llamada (barato vs ray tracing)
#[derive(Clone)]
struct GridLite { w:i32,h:i32,d:i32, min:Vector3, data:Vec<u8> }
#[inline] fn kind_to_u8(k: BlockKind) -> u8 {
    match k { BlockKind::Grass=>1, BlockKind::Dirt=>2, BlockKind::Stone=>3,
              BlockKind::Log=>4, BlockKind::Leaves=>5, BlockKind::Water=>6 }
}
#[inline] fn u8_to_kind(v: u8) -> Option<BlockKind> {
    match v {1=>Some(BlockKind::Grass),2=>Some(BlockKind::Dirt),3=>Some(BlockKind::Stone),
             4=>Some(BlockKind::Log),5=>Some(BlockKind::Leaves),6=>Some(BlockKind::Water),_=>None}
}
#[inline] fn gidx(g:&GridLite,x:i32,y:i32,z:i32)->usize {
    (y as usize)*(g.w as usize)*(g.d as usize) + (z as usize)*(g.w as usize) + (x as usize)
}
fn build_grid_lite(scene:&SceneRT)->GridLite{
    let mut miny=f32::INFINITY; let mut maxy=-f32::INFINITY;
    for b in &scene.blocks{ miny=miny.min(b.center.y); maxy=maxy.max(b.center.y); }
    let y0=(miny-0.5).floor()+0.5; let y1=(maxy+0.5).ceil()-0.5;
    let h=(y1-y0+1.0).max(1.0) as i32;
    let w=16i32; let d=16i32;
    let min = Vector3::new(-(w as f32)*0.5, y0-0.5, -(d as f32)*0.5);
    let mut data=vec![0u8;(w*d*h) as usize];
    for b in &scene.blocks {
        let ix=((b.center.x + (w as f32)*0.5).floor() as i32).clamp(0,w-1);
        let iy=((b.center.y - y0).round() as i32).clamp(0,h-1);
        let iz=((b.center.z + (d as f32)*0.5).floor() as i32).clamp(0,d-1);
        data[gidx(&GridLite{w,h,d,min,data:Vec::new()},ix,iy,iz)] = kind_to_u8(b.kind);
    }
    GridLite{w,h,d,min,data}
}
#[inline]
fn ray_aabb(o:Vector3, d:Vector3, mn:Vector3, mx:Vector3)->Option<(f32,f32)>{
    let inv=Vector3::new(1.0/d.x,1.0/d.y,1.0/d.z);
    let mut t0=(mn-o)*inv; let mut t1=(mx-o)*inv;
    if t0.x>t1.x{std::mem::swap(&mut t0.x,&mut t1.x);}
    if t0.y>t1.y{std::mem::swap(&mut t0.y,&mut t1.y);}
    if t0.z>t1.z{std::mem::swap(&mut t0.z,&mut t1.z);}
    let tmin=t0.x.max(t0.y.max(t0.z)); let tmax=t1.x.min(t1.y.min(t1.z));
    if tmax<tmin || tmax<=1e-4 {None} else {Some((tmin.max(1e-4),tmax))}
}

fn first_hit_fast(scene:&SceneRT, o:Vector3, d:Vector3) -> Option<(Hit, BlockKind)> {
    let g = build_grid_lite(scene);
    let max = g.min + Vector3::new(g.w as f32, g.h as f32, g.d as f32);
    let (mut t, tmax_all) = ray_aabb(o,d,g.min,max)?;
    let mut p = o + d*t;

    let mut ix=((p.x-g.min.x).floor() as i32).clamp(0,g.w-1);
    let mut iy=((p.y-g.min.y).floor() as i32).clamp(0,g.h-1);
    let mut iz=((p.z-g.min.z).floor() as i32).clamp(0,g.d-1);

    let (sx,sy,sz)=(if d.x>0.0{1}else{-1}, if d.y>0.0{1}else{-1}, if d.z>0.0{1}else{-1});

    let mut next_x = g.min.x + (if d.x>0.0 {(ix+1) as f32} else {ix as f32});
    let mut next_y = g.min.y + (if d.y>0.0 {(iy+1) as f32} else {iy as f32});
    let mut next_z = g.min.z + (if d.z>0.0 {(iz+1) as f32} else {iz as f32});

    let mut tmaxx= if d.x!=0.0 {(next_x-o.x)/d.x} else {f32::INFINITY};
    let mut tmaxy= if d.y!=0.0 {(next_y-o.y)/d.y} else {f32::INFINITY};
    let mut tmaxz= if d.z!=0.0 {(next_z-o.z)/d.z} else {f32::INFINITY};
    let tdx= if d.x!=0.0 {(1.0/d.x).abs()} else {f32::INFINITY};
    let tdy= if d.y!=0.0 {(1.0/d.y).abs()} else {f32::INFINITY};
    let tdz= if d.z!=0.0 {(1.0/d.z).abs()} else {f32::INFINITY};

    let mut face:u8=255;
    while t<=tmax_all {
        if ix>=0 && ix<g.w && iy>=0 && iy<g.h && iz>=0 && iz<g.d {
            let v=g.data[gidx(&g,ix,iy,iz)];
            if let Some(kind)=u8_to_kind(v){
                p=o+d*t;
                // normal/uv
                let n = match face {
                    0=>Vector3::new(-1.0,0.0,0.0), 1=>Vector3::new(1.0,0.0,0.0),
                    2=>Vector3::new(0.0,-1.0,0.0), 3=>Vector3::new(0.0,1.0,0.0),
                    4=>Vector3::new(0.0,0.0,-1.0), 5=>Vector3::new(0.0,0.0,1.0),
                    _=>Vector3::new(0.0,1.0,0.0)
                };
                let cx=g.min.x+ix as f32+0.5; let cy=g.min.y+iy as f32+0.5; let cz=g.min.z+iz as f32+0.5;
                let s=Vector3::new(p.x-(cx-0.5), p.y-(cy-0.5), p.z-(cz-0.5));
                let uv= match face {
                    0=>[1.0 - s.z, 1.0 - s.y],
                    1=>[s.z,       1.0 - s.y],
                    2=>[s.x,       s.z      ],
                    3=>[s.x,       1.0 - s.z],
                    4=>[s.x,       1.0 - s.y],
                    5=>[1.0 - s.x, 1.0 - s.y],
                    _=>[0.0,0.0]
                };

                if matches!(kind, BlockKind::Leaves) {
                    let (_c,a)=sample_block_linear_alpha(&scene.mats,uv,face,kind);
                    if a < 0.1 { /* continúa */ } else {
                        let hit=Hit{id:1,t,p,n,uv,face}; return Some((hit,kind));
                    }
                } else {
                    let hit=Hit{id:1,t,p,n,uv,face}; return Some((hit,kind));
                }
            }
        }
        if tmaxx <= tmaxy && tmaxx <= tmaxz {
            t=tmaxx; tmaxx+=tdx; ix+=sx; face= if sx==1 {0} else {1};
        } else if tmaxy <= tmaxz {
            t=tmaxy; tmaxy+=tdy; iy+=sy; face= if sy==1 {2} else {3};
        } else {
            t=tmaxz; tmaxz+=tdz; iz+=sz; face= if sz==1 {4} else {5};
        }
    }
    None
}

pub fn shadow_query_fast(scene:&SceneRT, p:Vector3, n:Vector3) -> bool {
    let eps=1e-3;
    let to_light = scene.light_pos - p;
    let dist_l = to_light.length();
    let d = to_light / dist_l;
    if let Some((hit, kind)) = first_hit_fast(scene, p + n*eps, d) {
        if hit.t < dist_l {
            return match kind {
                BlockKind::Leaves => {
                    // dither estable por texel para penumbra
                    let (_c,a)=sample_block_linear_alpha(&scene.mats, hit.uv, hit.face, kind);
                    let m = hash01(hit.uv[0]*64.0, hit.uv[1]*64.0, hit.face as f32);
                    a > m
                }
                BlockKind::Water => false,
                _ => true
            };
        }
    }
    false
}

pub fn shade_floor(_pre: &CamPre, scene: &SceneRT, hit: &Hit) -> Vector3 {
    let n = hit.n.normalized();
    let l = (scene.light_pos - hit.p).normalized();

    let k_wrap = 0.25;
    let diff = clamp01((n.dot(l) + k_wrap) / (1.0 + k_wrap));

    let in_shadow = shadow_query_fast(scene, hit.p, n);

    let ambient = 0.10;
    let mut c_lin = scene.floor_color * ambient;
    if !in_shadow {
        c_lin += scene.floor_color * diff;
    }
    gamma_encode(c_lin)
}
