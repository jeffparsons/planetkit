use piston_window::PistonWindow;

use window;
use app;
use slog;
use slog_term;

/// Create a new simple PlanetKit app and window.
///
/// Uses all default settings, and logs to standard output.
pub fn new() -> (app::App, PistonWindow) {
    // Set up logger to print to standard output.
    use slog::DrainExt;
    let drain = slog_term::streamer().compact().build().fuse();
    let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
    let log = root_log;

    let window = window::make_window(&log);
    let app = app::App::new(&log, &window);

    (app, window)
}
