use specs;
use slog::Logger;

use types::*;

/// Abstract game factory.
///
/// This is the primary interface that defines the behaviour of any
/// application built on PlanetKit, in that it defines what systems
/// and components will be set up and run.
pub trait Game {
    fn init_systems(
        &self,
        planner: &mut specs::Planner<TimeDelta>,
        log: &Logger,
    );
}
