extern crate specs;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate planetkit as pk;


#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;



use gfx::traits::FactoryExt;
use gfx::Device;
use glutin::GlContext;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        color: [f32; 3] = "a_Color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}



const TRIANGLE: [Vertex; 3] = [
    Vertex { pos: [ -0.5, -0.5 ], color: [1.0, 0.0, 0.0] },
    Vertex { pos: [  0.5, -0.5 ], color: [0.0, 1.0, 0.0] },
    Vertex { pos: [  0.0,  0.5 ], color: [0.0, 0.0, 1.0] }
];

const CLEAR_COLOR: [f32; 4] = [0.1, 0.2, 0.3, 1.0];


fn main() {
    stdweb::initialize();

    // Print evidence that we managed to get _something_ to run.
    let globe = pk::globe::Globe::new_earth_scale_example();
    println!("Globe size: {}", globe.spec().floor_radius);

    // Create a world with a dispatcher.
    use pk::simple;
    use pk::types::TimeDeltaResource;
    use pk::globe::Globe;


    let (mut app, mut window) = simple::new_populated(simple::noop_create_systems);
    app.run(&mut window);


    // TEMP: will webgl work for realsies?
    
    

    // let (
    //     log,
    //     world,
    //     dispatcher_builder,
    //     movement_input_adapter,
    //     mining_input_adapter,
    // ) = simple::new_populated_without_window(simple::noop_create_systems);

    // println!("after create world, before create window");



    // // ... Well, this seems to work. What is it that Piston is doing for us, again? :P
    // // Time to start a 'web' module in planetkit and build a window based on this.




    // let mut events_loop = glutin::EventsLoop::new();
    // let window_config = glutin::WindowBuilder::new()
    //     .with_title("Triangle example".to_string())
    //     .with_dimensions(800, 600);


    // let (api, version, vs_code, fs_code) = (
    //     glutin::Api::WebGl, (2, 0),
    //     include_bytes!("shaders/triangle_300_es.glslv").to_vec(),
    //     include_bytes!("shaders/triangle_300_es.glslf").to_vec(),
    // );

    // let context = glutin::ContextBuilder::new()
    //     .with_gl(glutin::GlRequest::Specific(api, version))
    //     .with_vsync(true);
    // let (window, mut device, mut factory, main_color, mut main_depth) =
    //     gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_config, context, &events_loop);
    // let mut encoder = gfx::Encoder::from(factory.create_command_buffer());

    // let pso = factory.create_pipeline_simple(&vs_code, &fs_code, pipe::new())
    //     .unwrap();
    // let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&TRIANGLE, ());
    // let mut data = pipe::Data {
    //     vbuf: vertex_buffer,
    //     out: main_color
    // };


    // events_loop.run_forever(move |event| {
    //     use glutin::{ControlFlow, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

    //     if let Event::WindowEvent { event, .. } = event {
    //         match event {
    //             WindowEvent::Closed |
    //             WindowEvent::KeyboardInput {
    //                 input: KeyboardInput {
    //                     virtual_keycode: Some(VirtualKeyCode::Escape),
    //                     ..
    //                 },
    //                 ..
    //             } => return ControlFlow::Break,
    //             WindowEvent::Resized(width, height) => {
    //                 window.resize(width, height);
    //                 gfx_window_glutin::update_views(&window, &mut data.out, &mut main_depth);
    //             },
    //             _ => (),
    //         }
    //     }

    //     // draw a frame
    //     encoder.clear(&data.out, CLEAR_COLOR);
    //     encoder.draw(&slice, &pso, &data);
    //     encoder.flush(&mut device);
    //     window.swap_buffers().unwrap();
    //     device.cleanup();

    //     ControlFlow::Continue
    // });


}
