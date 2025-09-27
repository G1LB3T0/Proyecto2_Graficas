mod camera;
mod geometry;
mod raytracer;
mod world;
mod light;
mod hud;

use raylib::prelude::*;
use camera::OrbitCamRT;
use raytracer::SceneRT;
use world::{Materials, Block, BlockKind};
use light::LightRig;
use hud::Hud;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("raycube_raytrace - CPU Ray Tracing (Rust + raylib + image)")
        .msaa_4x()
        .build();
    rl.set_target_fps(60);

    // Resolución del render (F1 alterna entre 1/2 y 1x)
    let mut half_res = true;

    // --------- CARGA MATERIALES ----------
    let mats = Materials {
        grass_top: image::open("assets/grasstop.png").expect("Falta grasstop.png").to_rgba8(),
        grass_side: image::open("assets/grass.png").expect("Falta grass.png").to_rgba8(),
        dirt: image::open("assets/dirt.png").expect("Falta dirt.png").to_rgba8(),
        stone: image::open("assets/stone.png").expect("Falta stone.png").to_rgba8(),
        log_side: image::open("assets/log_side.png").expect("Falta log_side.png").to_rgba8(),
        log_top: image::open("assets/log_top.png").expect("Falta log_top.png").to_rgba8(),
        leaves: image::open("assets/leaves.png").expect("Falta leaves.png").to_rgba8(),
        water: image::open("assets/water.png").expect("Falta water.png").to_rgba8(),
    };

    // --------- CARGA CAPAS (16x16) ----------
    let mut blocks = world::load_layers_dir("assets/layers", "layer_", 16, 16);

    // Si aún no tienes capas, deja un bloque demo
    if blocks.is_empty() {
        blocks.push(Block { center: Vector3::new(0.0, 0.5, 0.0), half: 0.5, kind: BlockKind::Grass });
        eprintln!("TIP: crea archivos assets/layers/layer_00.txt, layer_01.txt, ... (16x16).");
    }

    // Cámara / escena
    let cam = OrbitCamRT::new(Vector3::new(0.0, 0.5, 0.0), 1280.0/720.0);
    let mut scene = SceneRT {
        cam,
        light_pos: Vector3::new(3.0, 4.0, 2.0),
        floor_color: Vector3::new(0.06, 0.07, 0.08), // ignorado si show_floor=false
        show_floor: false,                            // ← piso desactivado
        blocks,
        mats,
    };

    // Luz y HUD
    let mut light_rig = LightRig::from_position(Vector3::new(0.0, 0.5, 0.0), scene.light_pos);
    let mut hud = Hud::new();

    // Texture destino donde subiremos el render de CPU
    let (mut tex_w, mut tex_h) = if half_res { (640, 360) } else { (1280, 720) };
    let mut rimg = Image::gen_image_color(tex_w, tex_h, Color::BLACK);
    let mut rtex = rl.load_texture_from_image(&thread, &rimg).unwrap();

    while !rl.window_should_close() {
        // ---- INPUT ----
        scene.cam.apply_input(&rl);
        hud.update_input(&rl);

        // Luz: rotación/orbita
        let dt = rl.get_frame_time();
        light_rig.update_input(&rl, dt);
        scene.light_pos = light_rig.position();

        // Resolución
        if rl.is_key_pressed(KeyboardKey::KEY_F1) {
            half_res = !half_res;
            (tex_w, tex_h) = if half_res { (640, 360) } else { (1280, 720) };
            rimg = Image::gen_image_color(tex_w, tex_h, Color::BLACK);
            rtex = rl.load_texture_from_image(&thread, &rimg).unwrap();
        }

        // ---- RENDER CPU (multihilo) ----
        let img = raytracer::render_mt(&scene, tex_w as u32, tex_h as u32);

        // Subir al Texture2D (tu build usa 1 argumento; maneja Result)
        if let Err(e) = rtex.update_texture(img.as_raw()) {
            eprintln!("update_texture error: {e:?}");
        }

        // ---- ESCALA Y ASPECT (antes de begin_drawing) ----
        let sw: f32 = rl.get_screen_width()  as f32;
        let sh: f32 = rl.get_screen_height() as f32;
        let sx: f32 = sw / tex_w as f32;
        let sy: f32 = sh / tex_h as f32;
        let scale: f32 = sx.min(sy);

        // Mantén la cámara con aspect correcto del viewport
        scene.cam.aspect = sw / sh;

        // ---- DRAW ----
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::RAYWHITE);

        d.draw_texture_ex(&rtex, Vector2::new(0.0, 0.0), 0.0, scale, Color::WHITE);

        // HUD (toggle con H si tu hud.rs lo soporta)
        hud.draw(&mut d);
    }
}
