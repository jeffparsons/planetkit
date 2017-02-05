use piston_window::Event;

/// Handles Piston input events and dispatches them to systems.
pub trait InputAdapter {
    fn handle(&self, event: &Event);
}
