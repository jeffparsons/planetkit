use piston_window::PistonWindow;
use glutin_window::GlutinWindow;
use slog::Logger;

pub fn make_window(log: &Logger) -> PistonWindow {
    use opengl_graphics::OpenGL;
    use piston::window::WindowSettings;
    use piston::window::AdvancedWindow;
    use piston_window::PistonWindow;


    println!("creating main window");


    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;



    let settings = WindowSettings::new("planetkit", [800, 600])
        .opengl(opengl)
        .exit_on_esc(true);


    println!("made settings");

    let samples = settings.get_samples();

    println!("got samples...");

    let glutin_window: GlutinWindow =
        settings.build().expect("failed to build settings...");


    println!("built settings");

    let mut window = PistonWindow::new(opengl, samples, glutin_window);



    window.set_capture_cursor(false);
    debug!(log, "Main window created");

    window
}
