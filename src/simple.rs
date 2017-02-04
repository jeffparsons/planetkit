use piston_window::PistonWindow;

use slog;
use slog_term;

use window;
use app;

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
    let mut app = app::App::new(&log, &window);

    {
        let planner = app.planner();

        // TODO: move _all_ other system initialization from `app.rs`
        // into here, and then back out into helper functions.

        // TODO: figure out how to deal with system priorities;
        // initially it'll just have to be hard-coded in every
        // example.

        use cell_dweller;
        let physics_sys = cell_dweller::PhysicsSystem::new(
            &log,
            0.1, // Seconds between falls
        );
        planner.add_system(physics_sys, "cd_physics", 90);

        use globe;
        let chunk_view_sys = globe::ChunkViewSystem::new(
            &log,
            0.05, // Seconds between geometry creation
        );
        planner.add_system(chunk_view_sys, "chunk_view", 50);
    }

    (app, window)
}
