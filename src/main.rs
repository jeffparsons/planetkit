extern crate rand;
extern crate noise;
extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate piston_window;
extern crate camera_controllers;
extern crate vecmath;
extern crate shader_version;
extern crate nalgebra as na;

// TODO: move most of these into specific functions
use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use opengl_graphics::OpenGL;
use piston_window::PistonWindow;
use gfx::pso::bundle::Bundle;

mod icosahedron;
mod globe;
mod chunk;

// Most of this boilerplate stuff adapted from
// <https://github.com/PistonDevelopers/piston-examples/blob/master/src/cube.rs>.
// TODO: This is incomprehensible! Do something about it.

gfx_vertex_struct!(
    _Vertex {
        a_pos: [f32; 4] = "a_pos",
        tex_coord: [f32; 2] = "a_tex_coord",
        a_color: [f32; 3] = "a_color",
    }
);

pub type Vertex = _Vertex;

impl Vertex {
    fn new(pos: [f32; 3], color: [f32; 3]) -> Vertex {
        Vertex {
            a_pos: [pos[0], pos[1], pos[2], 1.0],
            a_color: color,
            tex_coord: [0.0, 0.0],
        }
    }
}

gfx_pipeline!(
    pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        u_model_view_proj: gfx::Global<[[f32; 4]; 4]> = "u_model_view_proj",
        t_color: gfx::TextureSampler<[f32; 4]> = "t_color",
        out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
);

pub struct App {
    t: f64,
    bundle: Bundle<
        gfx_device_gl::Resources,
        pipe::Data<gfx_device_gl::Resources>
    >,
    model: vecmath::Matrix4<f32>,
    projection: [[f32; 4]; 4],
    first_person: camera_controllers::FirstPerson,
}

impl App {
    fn render(&mut self, args: &RenderArgs, window: &mut PistonWindow) {
        const CLEAR_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];
        window.encoder.clear(&window.output_color, CLEAR_COLOR);
        window.encoder.clear_depth(&window.output_stencil, 1.0);

        // Draw the globe.
        self.bundle.data.u_model_view_proj = camera_controllers::model_view_projection(
            self.model,
            self.first_person.camera(args.ext_dt).orthogonal(),
            self.projection
        );
        window.encoder.draw(
            &self.bundle.slice,
            &self.bundle.pso,
            &self.bundle.data
        );
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;
    }
}

fn main() {
    use piston::window::Window;
    use piston::window::AdvancedWindow;
    use gfx::traits::*;
    use camera_controllers::{
        FirstPersonSettings,
        FirstPerson,
        CameraPerspective,
    };
    use shader_version::Shaders;
    use shader_version::glsl::GLSL;

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: PistonWindow = WindowSettings::new(
        "black-triangle",
        [640, 480]
    )
        .opengl(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();
    window.set_capture_cursor(false);

    let globe = globe::Globe::new_example();
    let (vertices, indices) = globe.make_geometry();
    let index_data: &[u16] = indices.as_slice();

    // Make OpenGL resource factory.
    // We'll use this for creating all our
    // vertex buffers, etc.
    let ref mut factory = window.factory.clone();

    let (vbuf, slice) = factory.create_vertex_buffer_with_slice(
        &vertices, index_data
    );

    let texels = [[0x20, 0xA0, 0xC0, 0x00]];
    let (_, texture_view) = factory.create_texture_const::<gfx::format::Rgba8>(
        gfx::tex::Kind::D2(1, 1, gfx::tex::AaMode::Single),
        &[&texels]).unwrap();

    let sinfo = gfx::tex::SamplerInfo::new(
        gfx::tex::FilterMethod::Bilinear,
        gfx::tex::WrapMode::Clamp
    );

    let glsl = opengl.to_glsl();
    let pso = factory.create_pipeline_simple(
        Shaders::new()
            .set(GLSL::V1_50, include_str!("shaders/copypasta_150.glslv"))
            .get(glsl).unwrap().as_bytes(),
        Shaders::new()
            .set(GLSL::V1_50, include_str!("shaders/copypasta_150.glslf"))
            .get(glsl).unwrap().as_bytes(),
        pipe::new()
    ).unwrap();

    let get_projection = |w: &PistonWindow| {
        let draw_size = w.window.draw_size();
        CameraPerspective {
            fov: 90.0, near_clip: 0.1, far_clip: 1000.0,
            aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32)
        }.projection()
    };

    let model: vecmath::Matrix4<f32> = vecmath::mat4_id();
    let projection = get_projection(&window);
    let first_person = FirstPerson::new(
        [0.5, 0.5, 4.0],
        FirstPersonSettings::keyboard_wasd()
    );

    let data = pipe::Data {
        vbuf: vbuf.clone(),
        u_model_view_proj: [[0.0; 4]; 4],
        t_color: (texture_view, factory.create_sampler(sinfo)),
        out_color: window.output_color.clone(),
        out_depth: window.output_stencil.clone(),
    };

    let bundle = Bundle::new(slice, pso, data);

    let mut app = App {
        t: 0.0,
        bundle: bundle,
        model: model,
        projection: projection,
        first_person: first_person,
    };

    let mut events = window.events();
    while let Some(e) = events.next(&mut window) {
        app.first_person.event(&e);

        window.draw_3d(&e, |mut window| {
            let args = e.render_args().unwrap();
            app.render(&args, &mut window);
        });

        if let Some(_) = e.resize_args() {
            app.projection = get_projection(&window);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
    }
}
