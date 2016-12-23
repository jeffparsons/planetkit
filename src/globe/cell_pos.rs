use globe::{ IntCoord, Root };

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CellPos {
    pub root: Root,
    pub x: IntCoord,
    pub y: IntCoord,
    pub z: IntCoord,
}

impl CellPos {
    // Convenience methods, mostly for testing.
    // This is kind of like the builder pattern
    // in that it allows chaining by returning self.
    //
    // I toyed with using a proper builder for this
    // but its use was just too verbose to justify.
    pub fn set_x(mut self, new_x: IntCoord) -> Self {
        self.x = new_x;
        self
    }
}
