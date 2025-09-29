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

fn nearly(a: f32, b: f32, eps: f32) -> bool { (a - b).abs() <= eps }
fn v_eq(a: Vector3, b: Vector3, eps: f32) -> bool {
    nearly(a.x,b.x,eps) && nearly(a.y,b.y,eps) && nearly(a.z,b.z,eps)
}
fn water_mode_eq(a: WaterMode, b: WaterMode) -> bool {
    matches!((a,b),
        (WaterMode::Off, WaterMode::Off) |
        (WaterMode::SkyOnly, WaterMode::SkyOnly) |
        (WaterMode::ReflectOnce, WaterMode::ReflectOnce)
    )
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("Proyecto2 Gráficas — CPU Ray Tracing")
        .msaa_4x()
        .build();
    rl.set_target_fps(60);

    // -------- resolución del render --------
    let mut half_res = true;  // empezar en resolución baja para mejor rendimiento

    // -------- materiales --------
    let mats = world::Materials {
        grass_top:  image::open("assets/grasstop.png").expect("Falta assets/grasstop.png").to_rgba8(),
        grass_side: image::open("assets/grass.png").expect("Falta assets/grass.png").to_rgba8(),
        dirt:       image::open("assets/dirt.png").expect("Falta assets/dirt.png").to_rgba8(),
        stone:      image::open("assets/stone.png").expect("Falta assets/stone.png").to_rgba8(),
        log_side:   image::open("assets/log_side.png").expect("Falta assets/log_side.png").to_rgba8(),
        log_top:    image::open("assets/log_top.png").expect("Falta assets/log_top.png").to_rgba8(),
        leaves:     image::open("assets/leaves.png").expect("Falta assets/leaves.png").to_rgba8(),
        water:      image::open("assets/water.png").expect("Falta assets/water.png").to_rgba8(),
        lamp_off:   image::open("assets/lamp_off.png").expect("Falta assets/lamp_off.png").to_rgba8(),
        lamp_on:    image::open("assets/lamp_on.png").expect("Falta assets/lamp_on.png").to_rgba8(),
    };

    // -------- capas -> bloques --------
    let mut blocks = world::load_layers_dir("assets/layers", "layer_", 16, 16);
    if blocks.is_empty() {
        blocks.push(world::Block {
            center: Vector3::new(0.0, 0.5, 0.0),
            half: 0.5,
            kind: world::BlockKind::Grass,
        });
        eprintln!("TIP: crea assets/layers/layer_00.txt (16x16) y sucesivos layer_01.txt, ...");
    }

    // acomodar isla (techo ≈ 1.2)
    {
        let mut maxy = -f32::INFINITY;
        for b in &blocks { maxy = maxy.max(b.center.y); }
        let target_top_y: f32 = 1.2;
        let dy = target_top_y - maxy;
        world::translate_blocks_y(&mut blocks, dy);
    }

    // -------- escena --------
    let cam = OrbitCamRT::new(Vector3::new(0.0, 0.5, 0.0), 1280.0/720.0);
    let mut scene = SceneRT {
        cam,
        light_pos: Vector3::new(3.0, 4.0, 2.0),
        floor_color: Vector3::new(0.06, 0.07, 0.08),
        show_floor: false,
        blocks,
        mats,
        water_mode: WaterMode::Off,  // cambiar a Off para mejor rendimiento inicial
        is_night: false,  // empezar en modo día
    };

    // -------- LUZ + HUD --------
    let mut light_rig = LightRig::from_position(Vector3::new(0.0, 0.5, 0.0), scene.light_pos);
    light_rig.min_radius = world::suggest_min_light_radius(16, 16, &scene.blocks);

    let mut hud = hud::Hud::new();

    // -------- render target --------
    let (mut tex_w, mut tex_h) = if half_res { (320, 180) } else { (640, 360) };  // resoluciones más bajas
    let mut rimg = Image::gen_image_color(tex_w, tex_h, Color::BLACK);
    let mut rtex = rl.load_texture_from_image(&thread, &rimg).unwrap();

    // cache "dirty" para CPU + frame skipping para optimización
    let mut last_eye    = scene.cam.eye();
    let mut last_target = scene.cam.target;
    let mut last_light  = scene.light_pos;
    let mut last_mode   = scene.water_mode;
    let mut last_is_night = scene.is_night;  // nuevo cache para día/noche
    let mut last_wh     = (tex_w, tex_h);
    let mut rtex_has_image = false;
    let mut frame_skip_counter = 0u32;  // contador para skipping de frames

    while !rl.window_should_close() {
        // ===== INPUT =====
        let (moved_blocks, _moved_light) = {
            let mut moved_blocks = false;
            let mut moved_light = false;

            // control de cámara
            scene.cam.apply_input(&rl);

            // control de luz (optimizado)
            let dt = rl.get_frame_time();
            let prev_light = light_rig.position();
            light_rig.update_input(&rl, dt);
            let new_light = light_rig.position();
            moved_light = !v_eq(prev_light, new_light, 0.01);  // epsilon más grande
            if moved_light {
                scene.light_pos = new_light;
            }

            // subir/bajar isla Z/X
            if rl.is_key_pressed(KeyboardKey::KEY_Z) {
                world::translate_blocks_y(&mut scene.blocks, -0.1);
                moved_blocks = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_X) {
                world::translate_blocks_y(&mut scene.blocks, 0.1);
                moved_blocks = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_C) {
                // reset height (acomodar techo ≈ 1.2)
                let mut maxy = -f32::INFINITY;
                for b in &scene.blocks { maxy = maxy.max(b.center.y); }
                let dy = 1.2 - maxy;
                world::translate_blocks_y(&mut scene.blocks, dy);
                moved_blocks = true;
            }

            (moved_blocks, moved_light)
        };
        
        if moved_blocks {
            light_rig.min_radius = world::suggest_min_light_radius(16,16,&scene.blocks);
        }

        // toggles
        if rl.is_key_pressed(KeyboardKey::KEY_F1) {
            half_res = !half_res;
            (tex_w, tex_h) = if half_res { (320, 180) } else { (640, 360) };  // resoluciones más bajas
            rimg = Image::gen_image_color(tex_w, tex_h, Color::BLACK);
            rtex = rl.load_texture_from_image(&thread, &rimg).unwrap();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F6) {
            scene.water_mode = match scene.water_mode {
                WaterMode::Off => WaterMode::SkyOnly,
                WaterMode::SkyOnly => WaterMode::ReflectOnce,
                WaterMode::ReflectOnce => WaterMode::Off,
            };
        }
        // Nuevo: Toggle día/noche con F5
        if rl.is_key_pressed(KeyboardKey::KEY_F5) {
            scene.is_night = !scene.is_night;
        }

        // mantener aspect
        let sw_i: i32 = rl.get_screen_width();
        let sh_i: i32 = rl.get_screen_height();
        let sw: f32 = sw_i as f32;
        let sh: f32 = sh_i as f32;
        scene.cam.aspect = sw / sh;

        // obtener FPS antes del borrowing mutable
        let fps = rl.get_fps();

        // DRAW
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::RAYWHITE);

        // === CPU Ray Tracing (optimizado con frame skipping) ===
        let eye = scene.cam.eye();
        let tgt = scene.cam.target;
        let eps = 0.001;  // epsilon más grande para menos renders innecesarios
        let cam_changed   = !v_eq(eye, last_eye, eps) || !v_eq(tgt, last_target, eps);
        let light_changed = !v_eq(scene.light_pos, last_light, eps);
        let wh_changed    = last_wh != (tex_w, tex_h);
        let mode_changed  = !water_mode_eq(last_mode, scene.water_mode);
        let night_changed = last_is_night != scene.is_night;  // detectar cambio día/noche
        let dirty = cam_changed || light_changed || moved_blocks || wh_changed || mode_changed || night_changed || !rtex_has_image;
        
        // Frame skipping para mejor rendimiento - renderizar solo cada pocos frames si no hay cambios grandes
        frame_skip_counter += 1;
        let should_render = dirty || (frame_skip_counter % 3 == 0);  // renderizar cada 3 frames si no hay cambios
        
        if should_render {
            let img = raytracer::render_mt(&scene, tex_w as u32, tex_h as u32);
            let _ = rtex.update_texture(img.as_raw());
            rtex_has_image = true;
            last_eye = eye; last_target = tgt; last_light = scene.light_pos; last_wh = (tex_w, tex_h); last_mode = scene.water_mode; last_is_night = scene.is_night;
            frame_skip_counter = 0;  // reset counter después de renderizar
        }

        // Optimizar scaling - usar nearest neighbor para mejor rendimiento
        let sx: f32 = sw / tex_w as f32;
        let sy: f32 = sh / tex_h as f32;
        let scale: f32 = sx.min(sy);
        if rtex_has_image {
            d.draw_texture_ex(&rtex, Vector2::new(0.0, 0.0), 0.0, scale, Color::WHITE);
        }

        // HUD
        hud.begin_frame();
        hud.line_col_size(format!("{} FPS", fps), Color::RED, 24);
        let res_label = if half_res { "Low (F1)" } else { "Med (F1)" };  // actualizar labels
        hud.line(format!("RT Res: {}x{}  {}", tex_w, tex_h, res_label));
        let day_night_str = if scene.is_night { "Night Mode (F5)" } else { "Day Mode (F5)" };
        hud.line(day_night_str);
        let mode_str = match scene.water_mode {
            WaterMode::Off => "Water: OFF",
            WaterMode::SkyOnly => "Water: SkyOnly (fast)",
            WaterMode::ReflectOnce => "Water: ReflectOnce (slower)",
        };
        hud.line(mode_str);
        hud.line("F5: día/noche  |  F6: toggle water reflections");
        hud.line("Mouse L drag: orbit  |  Wheel: zoom  |  R: reset cámara");
        hud.line("J/L yaw luz  |  I/K pitch  |  U/O radio  |  P spin  |  T reset luz");
        hud.line("Z/X bajar/subir isla  |  C reset altura  |  H mostrar/ocultar HUD");
        hud.draw(&mut d);
    }
}