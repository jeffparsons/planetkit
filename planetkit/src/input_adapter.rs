use piston::input::Input;

/// Handles Piston input events and dispatches them to systems.
pub trait InputAdapter {
    fn handle(&self, input_event: &Input);
}
