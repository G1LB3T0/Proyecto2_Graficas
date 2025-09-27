use std::{fs, path::Path};
use raylib::core::math::Vector3;
use image::RgbaImage;

// ----------- Tipos de bloque / material -----------
#[derive(Clone, Copy, Debug)]
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
// Busca layer_00.txt, layer_01.txt, ... hasta que falte uno.
pub fn load_layers_dir(dir: &str, prefix: &str, grid_w: usize, grid_h: usize) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut y = 0usize;

    loop {
        let fname = format!("{prefix}{:02}.txt", y);
        let path = Path::new(dir).join(fname);
        if !path.exists() {
            // Si no hay ni layer_00.txt, termina; si ya cargamos algunas, detén el loop.
            if y == 0 { eprintln!("WARN: No se encontró ninguna capa en {dir}/ (esperaba {prefix}00.txt)"); }
            break;
        }
        let txt = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => { eprintln!("No pude leer {}: {e}", path.display()); break; }
        };

        let mut rows: Vec<Vec<char>> = Vec::new();
        for line in txt.lines() {
            if line.trim().is_empty() { continue; } // ignora líneas vacías
            rows.push(line.chars().collect());
        }
        if rows.len() < grid_h {
            eprintln!("WARN: {} tiene menos de {} filas; rellenando aire.", path.display(), grid_h);
        }

        for z in 0..grid_h {
            let row = rows.get(z).cloned().unwrap_or_default();
            for x in 0..grid_w {
                let ch = row.get(x).copied().unwrap_or(' ');
                if let Some(kind) = char_to_kind(ch) {
                    // Centro del bloque; centramos el grid alrededor del origen
                    let cx = (x as f32 + 0.5) - grid_w as f32 * 0.5;
                    let cy = (y as f32 + 0.5);
                    let cz = (z as f32 + 0.5) - grid_h as f32 * 0.5;
                    blocks.push(Block {
                        center: Vector3::new(cx, cy, cz),
                        half: 0.5,
                        kind,
                    });
                }
            }
        }

        y += 1;
    }

    blocks
}

// Mapa de caracteres → tipo
fn char_to_kind(c: char) -> Option<BlockKind> {
    match c {
        'g' | 'G' => Some(BlockKind::Grass),
        'd' | 'D' => Some(BlockKind::Dirt),
        's' | 'S' => Some(BlockKind::Stone),
        'l' | 'L' => Some(BlockKind::Log),
        'v' | 'V' => Some(BlockKind::Leaves),
        'w' | 'W' => Some(BlockKind::Water),
        ' ' | '.' => None,                  // aire
        _ => None,                          // ignora otros
    }
}
