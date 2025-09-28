mod camera;
mod geometry;
mod raytracer; // carpeta src/raytracer/ con mod.rs, etc.
mod world;
mod light;
mod hud;

use raylib::prelude::*;
use camera::OrbitCamRT;
use raytracer::{SceneRT, WaterMode};
use light::LightRig;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("Proyecto2 Gráficas — CPU Ray Tracing (Rust + raylib + image)")
        .msaa_4x()
        .build();
    rl.set_target_fps(60);

    // -------- Resolución del render (F1 alterna) --------
    let mut half_res = true;

    // -------- CARGA MATERIALES --------
    let mats = world::Materials {
        grass_top: image::open("assets/grasstop.png").expect("Falta assets/grasstop.png").to_rgba8(),
        grass_side: image::open("assets/grass.png").expect("Falta assets/grass.png").to_rgba8(),
        dirt: image::open("assets/dirt.png").expect("Falta assets/dirt.png").to_rgba8(),
        stone: image::open("assets/stone.png").expect("Falta assets/stone.png").to_rgba8(),
        log_side: image::open("assets/log_side.png").expect("Falta assets/log_side.png").to_rgba8(),
        log_top: image::open("assets/log_top.png").expect("Falta assets/log_top.png").to_rgba8(),
        leaves: image::open("assets/leaves.png").expect("Falta assets/leaves.png").to_rgba8(),
        water: image::open("assets/water.png").expect("Falta assets/water.png").to_rgba8(),
    };

    // -------- CARGA CAPAS (16x16) -> solo superficie --------
    let mut blocks = world::load_layers_dir("assets/layers", "layer_", 16, 16);
    if blocks.is_empty() {
        // Bloque demo si aún no hay capas
        blocks.push(world::Block {
            center: Vector3::new(0.0, 0.5, 0.0),
            half: 0.5,
            kind: world::BlockKind::Grass,
        });
        eprintln!("TIP: crea assets/layers/layer_00.txt (16x16) y sucesivos layer_01.txt, ...");
    }

    // === Posición inicial de la isla (auto) ===
    let (mut _min_y, mut max_y) = (f32::INFINITY, -f32::INFINITY);
    for b in &blocks {
        _min_y = _min_y.min(b.center.y);
        max_y = max_y.max(b.center.y);
    }
    let target_top_y: f32 = 1.2;
    let mut y_offset: f32 = target_top_y - max_y;
    world::translate_blocks_y(&mut blocks, y_offset);

    // -------- Cámara / escena --------
    let cam = OrbitCamRT::new(Vector3::new(0.0, 0.5, 0.0), 1280.0 / 720.0);
    let mut scene = SceneRT {
        cam,
        light_pos: Vector3::new(3.0, 4.0, 2.0),
        floor_color: Vector3::new(0.06, 0.07, 0.08), // ignorado si show_floor=false
        show_floor: false,                           // piso oculto
        blocks,
        mats,
        water_mode: WaterMode::SkyOnly,              // ← default rápido
    };

    // -------- Luz orbital y HUD --------
    let mut light_rig = LightRig::from_position(Vector3::new(0.0, 0.5, 0.0), scene.light_pos);
    light_rig.min_radius = world::suggest_min_light_radius(16, 16, &scene.blocks);

    let mut hud = hud::Hud::new();

    // -------- Texture destino (render de CPU subido a GPU) --------
    let (mut tex_w, mut tex_h) = if half_res { (640, 360) } else { (1280, 720) };
    let mut rimg = Image::gen_image_color(tex_w, tex_h, Color::BLACK);
    let mut rtex = rl.load_texture_from_image(&thread, &rimg).unwrap();

    // -------- Control interactivo de altura --------
    // Z: baja, X: sube, C: reset a altura "cómoda"
    let y_speed: f32 = 2.0; // unidades por segundo

    while !rl.window_should_close() {
        // ---- INPUT ----
        scene.cam.apply_input(&rl);
        hud.update_input(&rl);

        let dt = rl.get_frame_time();
        light_rig.update_input(&rl, dt);
        scene.light_pos = light_rig.position();

        // Mover isla en Y
        let mut moved = false;
        if rl.is_key_down(KeyboardKey::KEY_Z) {
            let dy = -y_speed * dt; y_offset += dy;
            world::translate_blocks_y(&mut scene.blocks, dy); moved = true;
        }
        if rl.is_key_down(KeyboardKey::KEY_X) {
            let dy =  y_speed * dt; y_offset += dy;
            world::translate_blocks_y(&mut scene.blocks, dy); moved = true;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_C) {
            let mut maxy = -f32::INFINITY;
            for b in &scene.blocks { maxy = maxy.max(b.center.y); }
            let dy = target_top_y - maxy;
            if dy.abs() > 1e-6 {
                y_offset += dy;
                world::translate_blocks_y(&mut scene.blocks, dy);
                moved = true;
            }
        }
        if moved {
            light_rig.min_radius = world::suggest_min_light_radius(16, 16, &scene.blocks);
        }

        // Toggle resolución
        if rl.is_key_pressed(KeyboardKey::KEY_F1) {
            half_res = !half_res;
            (tex_w, tex_h) = if half_res { (640, 360) } else { (1280, 720) };
            rimg = Image::gen_image_color(tex_w, tex_h, Color::BLACK);
            rtex = rl.load_texture_from_image(&thread, &rimg).unwrap();
        }

        // Toggle modo agua (F6)
        if rl.is_key_pressed(KeyboardKey::KEY_F6) {
            scene.water_mode = match scene.water_mode {
                WaterMode::Off => WaterMode::SkyOnly,
                WaterMode::SkyOnly => WaterMode::ReflectOnce,
                WaterMode::ReflectOnce => WaterMode::Off,
            };
        }

        // ---- RENDER CPU (multihilo) ----
        let img = raytracer::render_mt(&scene, tex_w as u32, tex_h as u32);

        // Subir al Texture2D
        if let Err(e) = rtex.update_texture(img.as_raw()) {
            eprintln!("update_texture error: {e:?}");
        }

        // ---- ESCALA Y ASPECT (antes de begin_drawing) ----
        let sw: f32 = rl.get_screen_width() as f32;
        let sh: f32 = rl.get_screen_height() as f32;
        let sx: f32 = sw / tex_w as f32;
        let sy: f32 = sh / tex_h as f32;
        let scale: f32 = sx.min(sy);

        // Mantener aspect correcto para la cámara
        scene.cam.aspect = sw / sh;

        // ---- DRAW ----
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::RAYWHITE);
        d.draw_texture_ex(&rtex, Vector2::new(0.0, 0.0), 0.0, scale, Color::WHITE);

        // HUD + info agua
        hud.draw(&mut d);
        let mode_str = match scene.water_mode {
            WaterMode::Off => "Water: OFF",
            WaterMode::SkyOnly => "Water: SkyOnly (fast)",
            WaterMode::ReflectOnce => "Water: ReflectOnce (slower)",
        };
        d.draw_text(mode_str, 10, 52, 18, Color::BLUE);
        d.draw_text("F6: toggle water reflections", 10, 72, 18, Color::DARKBLUE);
        d.draw_text("Z/X: bajar/subir isla | C: reset altura", 10, 92, 18, Color::DARKGRAY);
        d.draw_fps(10, 10);
    }
}
