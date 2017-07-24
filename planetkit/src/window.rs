use piston_window::PistonWindow;
use slog::Logger;

pub fn make_window(log: &Logger) -> PistonWindow {
    use opengl_graphics::OpenGL;
    use piston::window::WindowSettings;
    use piston::window::AdvancedWindow;
    use piston_window::PistonWindow;

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    info!(log, "Creating main window");
    let mut window: PistonWindow = WindowSettings::new("planetkit", [800, 600])
        .opengl(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();
    window.set_capture_cursor(false);
    debug!(log, "Main window created");

    window
}
