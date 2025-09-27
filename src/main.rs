mod camera;
mod geometry;
mod scene;
mod light;
mod util;
mod raytracer;

use raylib::prelude::*;
use camera::OrbitCamRT;
use scene::SceneRT;
use light::LightRig;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("raycube_raytrace - CPU Ray Tracing (Rust + raylib + image)")
        .msaa_4x()
        .build();
    rl.set_target_fps(60);

    // Resolución del render (F1 alterna entre 1/2 y 1x)
    let mut half_res = true;

    // Textura del cubo (con fallback si falta en disco)
    let tex_img = util::texture::load_or_make_texture("assets/texture1.png");

    // Cámara / escena
    let cam = OrbitCamRT::new(Vector3::new(0.0, 0.5, 0.0), 1280.0/720.0);
    let mut scene = SceneRT {
        cam,
        light_pos: Vector3::new(3.0, 4.0, 2.0),
        floor_color: Vector3::new(0.18, 0.2, 0.23),
        cube_center: Vector3::new(0.0, 0.5, 0.0),
        cube_half: 0.5,
        tex: tex_img,
    };

    // Luz en coordenadas esféricas controlada por teclado
    let mut light_rig = LightRig::from_position(scene.cube_center, scene.light_pos);

    // Texture destino donde subiremos el render de CPU
    let (mut tex_w, mut tex_h) = if half_res { (640, 360) } else { (1280, 720) };
    let mut rimg = Image::gen_image_color(tex_w, tex_h, Color::BLACK);
    let mut rtex = rl.load_texture_from_image(&thread, &rimg).unwrap();

    while !rl.window_should_close() {
        // ---- INPUT ----
        scene.cam.apply_input(&rl);

        // Luz (J/L yaw, I/K pitch, U/O radio, P spin, T reset)
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

        // Subir al Texture2D (tu build usa 1 argumento; maneja Result para evitar warning)
        if let Err(e) = rtex.update_texture(img.as_raw()) {
            eprintln!("update_texture error: {e:?}");
        }
        // Si tu firma requiere &thread, usa:
        // if let Err(e) = rtex.update_texture(&thread, img.as_raw()) { eprintln!("update_texture error: {e:?}"); }

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

        d.draw_text("CPU RT | Mouse L: orbit cam | Wheel: zoom | R: reset cam | F1: res",
                    10, 10, 18, Color::BLACK);
        d.draw_text("Light: J/L yaw | I/K pitch | U/O radius | P spin | T reset",
                    10, 32, 18, Color::BLACK);
        d.draw_fps(10, 54);
    }
}
