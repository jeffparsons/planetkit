use std::sync::mpsc;

use shred;
use slog;
#[cfg(not(target_os = "emscripten"))]
use slog_async;
#[cfg(not(target_os = "emscripten"))]
use slog_term;
use specs;

use crate::app::App;
use crate::cell_dweller;
use crate::net::{GameMessage, ServerResource};
use crate::window;

/// Builder for [`App`].
///
/// Will eventually learn how to create different kinds of
/// application runners, including a CLI-only one that doesn't
/// need to include any renderering systems.
///
/// Contains some optional convenience functions for adding
/// commonly used systems.
#[must_use]
pub struct AppBuilder {
    root_log: slog::Logger,
    world: specs::World,
    dispatcher_builder: shred::DispatcherBuilder<'static, 'static>,
    // We may or may not create these, depending on the game.
    movement_input_adapter: Option<Box<cell_dweller::MovementInputAdapter>>,
    mining_input_adapter: Option<Box<cell_dweller::MiningInputAdapter>>,
}

impl AppBuilder {
    pub fn new() -> AppBuilder {
        use crate::LogResource;

        // Set up logger.
        // REVISIT: make logger configurable? E.g. based on whether on web or not.
        // Or just commit to a specific kind of drain for emscripten?
        #[cfg(not(target_os = "emscripten"))]
        let drain = {
            use slog::Drain;

            let decorator = slog_term::TermDecorator::new().build();
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            slog_async::Async::new(drain).build().fuse()
        };
        #[cfg(target_os = "emscripten")]
        let drain = slog::Discard;
        let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

        // Create world and register all component types.
        // TODO: move component type registration elsewhere;
        // AutoSystems that use them should ensure that they are registered.
        let mut world = specs::World::new();
        world.register::<crate::cell_dweller::CellDweller>();
        world.register::<crate::render::Visual>();
        world.register::<crate::Spatial>();
        world.register::<crate::physics::Velocity>();
        world.register::<crate::physics::Mass>();
        world.register::<crate::globe::Globe>();
        world.register::<crate::globe::ChunkView>();
        world.register::<crate::net::NetMarker>();

        // Initialize resources that can't implement `Default`.
        world.add_resource(LogResource::new(&root_log));

        // NOTE: You must opt in to having a `ServerResource`
        // if you want it by calling `with_networking`.

        AppBuilder {
            root_log,
            world,
            dispatcher_builder: specs::DispatcherBuilder::new(),
            movement_input_adapter: None,
            mining_input_adapter: None,
        }
    }

    pub fn build_gui(self) -> App {
        // TODO: move that function into this file; it doesn't need its own module.
        let window = window::make_window(&self.root_log);

        // TODO: hand the root log over to App, rather than making it borrow it.
        let mut app = App::new(&self.root_log, window, self.world, self.dispatcher_builder);
        if let Some(movement_input_adapter) = self.movement_input_adapter {
            app.add_input_adapter(movement_input_adapter);
        }
        if let Some(mining_input_adapter) = self.mining_input_adapter {
            app.add_input_adapter(mining_input_adapter);
        }
        app
    }

    pub fn with_systems<F: AddSystemsFn<'static, 'static>>(mut self, add_systems_fn: F) -> Self {
        self.dispatcher_builder =
            add_systems_fn(&self.root_log, &mut self.world, self.dispatcher_builder);
        self
    }

    // TODO: Remark (assert!) on how this must
    // be called before adding any networking-related systems.
    pub fn with_networking<G: GameMessage>(mut self) -> Self {
        self.world
            .add_resource(ServerResource::<G>::new(&self.root_log));
        self
    }

    /// Add a few systems that you're likely to want, especially if you're just getting
    /// started with PlanetKit and want to get up and running quickly.
    pub fn with_common_systems(mut self) -> Self {
        use crate::globe;

        // Set up input adapters.
        let (movement_input_sender, movement_input_receiver) = mpsc::channel();
        self.movement_input_adapter = Some(Box::new(cell_dweller::MovementInputAdapter::new(
            movement_input_sender,
        )));

        let (mining_input_sender, mining_input_receiver) = mpsc::channel();
        self.mining_input_adapter = Some(Box::new(cell_dweller::MiningInputAdapter::new(
            mining_input_sender,
        )));

        let movement_sys =
            cell_dweller::MovementSystem::new(movement_input_receiver, &self.root_log);

        let mining_sys = cell_dweller::MiningSystem::new(mining_input_receiver, &self.root_log);

        let cd_physics_sys = cell_dweller::PhysicsSystem::new(
            &self.root_log,
            0.1, // Seconds between falls
        );

        let chunk_sys = globe::ChunkSystem::new(&self.root_log);

        let chunk_view_sys = globe::ChunkViewSystem::new(
            &self.root_log,
            0.05, // Seconds between geometry creation
        );

        self.with_systems(
            |_logger: &slog::Logger,
             _world: &mut specs::World,
             dispatcher_builder: specs::DispatcherBuilder<'static, 'static>| {
                dispatcher_builder
                    // Try to get stuff most directly linked to input done first
                    // to avoid another frame of lag.
                    .with(movement_sys, "cd_movement", &[])
                    .with(mining_sys, "cd_mining", &["cd_movement"])
                    .with_barrier()
                    .with(cd_physics_sys, "cd_physics", &[])
                    .with(chunk_sys, "chunk", &[])
                    // Don't depend on chunk system; chunk view can lag happily, so we'd prefer
                    // to be able to run it in parallel.
                    .with(chunk_view_sys, "chunk_view", &[])
            },
        )
    }
}

pub trait AddSystemsFn<'a, 'b>:
    FnOnce(
    &slog::Logger,
    &mut specs::World,
    specs::DispatcherBuilder<'a, 'b>,
) -> specs::DispatcherBuilder<'a, 'b>
{
}

impl<'a, 'b, F> AddSystemsFn<'a, 'b> for F where
    F: FnOnce(
        &slog::Logger,
        &mut specs::World,
        specs::DispatcherBuilder<'a, 'b>,
    ) -> specs::DispatcherBuilder<'a, 'b>
{
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
