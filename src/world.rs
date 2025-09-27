use std::{fs, path::Path};
use raylib::core::math::Vector3;
use image::RgbaImage;

// ----------- Tipos de bloque / material -----------
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind { Grass, Dirt, Stone, Log, Leaves, Water }

#[derive(Clone, Debug)]
pub struct Block {
    pub center: Vector3,
    pub half: f32,       // 0.5 típico
    pub kind: BlockKind,
}

// Todas las texturas necesarias (las cargas en main.rs)
#[derive(Clone)]
pub struct Materials {
    pub grass_top: RgbaImage,
    pub grass_side: RgbaImage,
    pub dirt: RgbaImage,
    pub stone: RgbaImage,
    pub log_side: RgbaImage,
    pub log_top: RgbaImage,
    pub leaves: RgbaImage,
    pub water: RgbaImage,
}

// ----------- Loader de capas 16x16 ---------------
// Lee assets/layers/layer_00.txt, layer_01.txt, ... hasta que falte uno.
// Construye una grilla 16x16xL y devuelve **solo bloques de superficie**.
pub fn load_layers_dir(dir: &str, prefix: &str, grid_w: usize, grid_h: usize) -> Vec<Block> {
    let mut layers: Vec<Vec<Vec<Option<BlockKind>>>> = Vec::new(); // [y][z][x]

    // 1) Leer capas y normalizar a 16x16
    let mut y = 0usize;
    loop {
        let fname = format!("{prefix}{:02}.txt", y);
        let path = Path::new(dir).join(&fname);
        if !path.exists() {
            if y == 0 {
                eprintln!("WARN: No se encontró ninguna capa en {dir}/ (esperaba {fname})");
            }
            break;
        }
        let txt = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => { eprintln!("No pude leer {}: {e}", path.display()); break; }
        };
        let raw = normalize_to_grid(&txt, grid_w, grid_h);
        // map a Option<BlockKind>
        let mut layer: Vec<Vec<Option<BlockKind>>> = vec![vec![None; grid_w]; grid_h];
        for z in 0..grid_h {
            for x in 0..grid_w {
                layer[z][x] = char_to_kind(raw[z][x]);
            }
        }
        layers.push(layer);
        y += 1;
    }

    if layers.is_empty() { return Vec::new(); }

    // 2) Extraer SOLO bloques de superficie (si alguna de sus 6 caras da a aire / borde)
    let lw = grid_w as i32;
    let lh = layers.len() as i32;
    let ld = grid_h as i32;

    let mut blocks = Vec::new();
    for yy in 0..lh {
        for zz in 0..ld {
            for xx in 0..lw {
                let kind = layers[yy as usize][zz as usize][xx as usize];
                if kind.is_none() { continue; }
                let k = kind.unwrap();

                let neigh = [
                    (xx-1, yy,   zz),
                    (xx+1, yy,   zz),
                    (xx,   yy-1, zz),
                    (xx,   yy+1, zz),
                    (xx,   yy,   zz-1),
                    (xx,   yy,   zz+1),
                ];
                let mut exposed = false;
                for (nx, ny, nz) in neigh {
                    if nx < 0 || nx >= lw || ny < 0 || ny >= lh || nz < 0 || nz >= ld {
                        exposed = true; break;
                    }
                    if layers[ny as usize][nz as usize][nx as usize].is_none() {
                        exposed = true; break;
                    }
                }
                if !exposed { continue; }

                // Centro del bloque; grid centrado en X/Z, Y desde 0 hacia arriba
                let cx = (xx as f32 + 0.5) - grid_w as f32 * 0.5;
                let cy = (yy as f32 + 0.5);
                let cz = (zz as f32 + 0.5) - grid_h as f32 * 0.5;
                blocks.push(Block {
                    center: Vector3::new(cx, cy, cz),
                    half: 0.5,
                    kind: k,
                });
            }
        }
    }
    blocks
}

// ------------------- Normalización 16x16 -------------------
fn normalize_to_grid(txt: &str, grid_w: usize, grid_h: usize) -> Vec<Vec<char>> {
    let mut rows: Vec<Vec<char>> = Vec::with_capacity(grid_h);

    for line in txt.lines() {
        if line.trim_start().starts_with('#') { continue; }
        let mut row: Vec<char> = Vec::with_capacity(grid_w);
        for ch in line.chars() {
            if is_valid_symbol(ch) {
                row.push(ch);
                if row.len() == grid_w { break; }
            }
        }
        while row.len() < grid_w { row.push('.'); } // aire
        rows.push(row);
        if rows.len() == grid_h { break; }
    }
    while rows.len() < grid_h { rows.push(vec!['.'; grid_w]); }
    rows
}

fn is_valid_symbol(c: char) -> bool {
    matches!(c, 'g'|'G'|'d'|'D'|'s'|'S'|'l'|'L'|'v'|'V'|'w'|'W'|'.'|' ')
}

fn char_to_kind(c: char) -> Option<BlockKind> {
    match c {
        'g' | 'G' => Some(BlockKind::Grass),
        'd' | 'D' => Some(BlockKind::Dirt),
        's' | 'S' => Some(BlockKind::Stone),
        'l' | 'L' => Some(BlockKind::Log),
        'v' | 'V' => Some(BlockKind::Leaves),
        'w' | 'W' => Some(BlockKind::Water),
        '.' | ' ' => None,
        _ => None,
    }
}

/// Mueve todos los bloques en Y (positivo = arriba, negativo = abajo).
pub fn translate_blocks_y(blocks: &mut [Block], dy: f32) {
    for b in blocks {
        b.center.y += dy;
    }
}

/// Estima un radio mínimo para que la luz no atraviese el mundo.
pub fn suggest_min_light_radius(grid_w: usize, grid_h: usize, blocks: &[Block]) -> f32 {
    let top_y = blocks.iter().fold(0.0_f32, |m, b| m.max(b.center.y + b.half));
    let horiz = grid_w.max(grid_h) as f32 * 0.6;
    horiz.max(top_y + 2.0)
}
