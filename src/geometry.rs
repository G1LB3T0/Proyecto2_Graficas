use raylib::prelude::*;

#[derive(Clone, Copy)]
pub struct Ray {
    pub o: Vector3,
    pub d: Vector3,
}

#[derive(Clone, Copy)]
pub struct Hit {
    pub t: f32,
    pub p: Vector3,
    pub n: Vector3,
    pub uv: [f32; 2], // coord para textura
    pub id: i32,      // 0 piso, 1 cubo, -1 nada
    pub face: u8,     // 0:-X 1:+X 2:-Y 3:+Y 4:-Z 5:+Z
}

impl Hit {
    pub fn none() -> Self {
        Self {
            t: f32::INFINITY,
            p: Vector3::new(0.0, 0.0, 0.0),
            n: Vector3::new(0.0, 0.0, 0.0),
            uv: [0.0, 0.0],
            id: -1,
            face: 255,
        }
    }
}

// Intersección con el plano y = 0
pub fn hit_plane_y0(ray: Ray) -> Option<Hit> {
    if ray.d.y.abs() < 1e-5 { return None; }
    let t = -ray.o.y / ray.d.y;
    if t <= 1e-4 { return None; }
    let p = ray.o + ray.d * t;
    Some(Hit {
        t, p,
        n: Vector3::new(0.0, 1.0, 0.0),
        uv: [p.x, p.z],
        id: 0,
        face: 255
    })
}

// Intersección con cubo AABB centrado en c con half-extent he (0.5 p/ cubo unitario)
pub fn hit_aabb(ray: Ray, c: Vector3, he: f32) -> Option<Hit> {
    let min = c - Vector3::new(he, he, he);
    let max = c + Vector3::new(he, he, he);

    let inv = Vector3::new(1.0 / ray.d.x, 1.0 / ray.d.y, 1.0 / ray.d.z);

    let mut t1 = (min.x - ray.o.x) * inv.x;
    let mut t2 = (max.x - ray.o.x) * inv.x;
    let mut tmin = t1.min(t2);
    let mut tmax = t1.max(t2);

    t1 = (min.y - ray.o.y) * inv.y; t2 = (max.y - ray.o.y) * inv.y;
    tmin = tmin.max(t1.min(t2));
    tmax = tmax.min(t1.max(t2));

    t1 = (min.z - ray.o.z) * inv.z; t2 = (max.z - ray.o.z) * inv.z;
    tmin = tmin.max(t1.min(t2));
    tmax = tmax.min(t1.max(t2));

    if tmax <= tmin || tmax < 1e-4 { return None; }
    let t = if tmin > 1e-4 { tmin } else { tmax };
    let p = ray.o + ray.d * t;

    let eps = 1e-3;
    let (mut n, mut uv, mut face) = (Vector3::new(0.0, 1.0, 0.0), [0.0, 0.0], 5u8);

    if (p.x - min.x).abs() < eps {
        n = Vector3::new(-1.0, 0.0, 0.0);
        uv = [(p.z - min.z)/(max.z-min.z), (p.y - min.y)/(max.y-min.y)];
        face = 0;
    } else if (p.x - max.x).abs() < eps {
        n = Vector3::new(1.0, 0.0, 0.0);
        uv = [1.0 - (p.z - min.z)/(max.z-min.z), (p.y - min.y)/(max.y-min.y)];
        face = 1;
    } else if (p.y - min.y).abs() < eps {
        n = Vector3::new(0.0, -1.0, 0.0);
        uv = [(p.x - min.x)/(max.x-min.x), (p.z - min.z)/(max.z-min.z)];
        face = 2;
    } else if (p.y - max.y).abs() < eps {
        n = Vector3::new(0.0, 1.0, 0.0);
        uv = [(p.x - min.x)/(max.x-min.x), 1.0 - (p.z - min.z)/(max.z-min.z)];
        face = 3;
    } else if (p.z - min.z).abs() < eps {
        n = Vector3::new(0.0, 0.0, -1.0);
        uv = [1.0 - (p.x - min.x)/(max.x-min.x), (p.y - min.y)/(max.y-min.y)];
        face = 4;
    } else if (p.z - max.z).abs() < eps {
        n = Vector3::new(0.0, 0.0, 1.0);
        uv = [(p.x - min.x)/(max.x-min.x), (p.y - min.y)/(max.y-min.y)];
        face = 5;
    }

    Some(Hit { t, p, n, uv, id: 1, face })
}
