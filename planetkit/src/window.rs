use piston_window::PistonWindow;
use glutin_window::GlutinWindow;
use slog::Logger;

pub fn make_window(log: &Logger) -> PistonWindow {
    use opengl_graphics::OpenGL;
    use piston::window::WindowSettings;
    use piston::window::AdvancedWindow;
    use piston_window::PistonWindow;

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create Glutin settings.
    info!(log, "Creating Glutin window");
    let settings = WindowSettings::new("planetkit", [800, 600])
        .opengl(opengl)
        .exit_on_esc(true);

    // Create Glutin window from settings.
    info!(log, "Creating Glutin window");
    let samples = settings.get_samples();
    let glutin_window: GlutinWindow = settings.build().expect("Failed to build Glutin window");

    // Create a Piston window.
    info!(log, "Creating main PistonWindow");
    let mut window: PistonWindow = PistonWindow::new(opengl, samples, glutin_window);

    window.set_capture_cursor(false);
    debug!(log, "Main window created");

    window
}
