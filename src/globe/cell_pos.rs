use globe::{ IntCoord, Root, RootIndex };

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
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
    //
    // TODO: move them into a special module that you
    // only import in tests?

    pub fn set_root(mut self, new_root_index: RootIndex) -> Self {
        self.root.index = new_root_index;
        self
    }

    pub fn set_x(mut self, new_x: IntCoord) -> Self {
        self.x = new_x;
        self
    }

    pub fn set_y(mut self, new_y: IntCoord) -> Self {
        self.y = new_y;
        self
    }

    pub fn set_z(mut self, new_z: IntCoord) -> Self {
        self.z = new_z;
        self
    }
}
