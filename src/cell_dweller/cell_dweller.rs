use specs;

use ::types::*;
use globe::{ CellPos, Dir, Spec };
use globe::movement::*;

pub struct CellDweller {
    // TODO: make these private and use guts trait pattern to expose them internally.
    // TODO: is this guts pattern worth a separate macro crate of its own?
    pos: CellPos,
    dir: Dir,
    // Most `CellDweller`s will also be `Spatial`s. Track the version of the globe-space
    // transform and the computed real-space transform so we know when the latter is dirty.
    globe_space_transform_version: u64,
    real_space_transform_version: u64,
    globe_spec: Spec,
}

impl CellDweller {
    pub fn new(pos: CellPos, dir: Dir, globe_spec: Spec) -> CellDweller {
        CellDweller {
            pos: pos,
            dir: dir,
            globe_space_transform_version: 1,
            real_space_transform_version: 0,
            globe_spec: globe_spec,
        }
    }

    pub fn pos(&self) -> CellPos {
        self.pos
    }

    pub fn set_cell_pos(&mut self, new_pos: CellPos) {
        self.pos = new_pos;
        self.globe_space_transform_version += 1;
    }

    /// Temporary function for testing until we have actual
    /// movement on the grid. This just adds one to the x-coordinate
    /// of the current position.
    pub fn temp_advance_pos(&mut self) {
        move_forward(
            &mut self.pos,
            &mut self.dir,
            self.globe_spec.root_resolution,
        ).unwrap();
        self.globe_space_transform_version += 1;
    }

    /// Calculate position in real-space.
    fn real_pos(&self) -> Pt3 {
        self.globe_spec.cell_bottom_center(self.pos)
    }

    pub fn is_real_space_transform_dirty(&self) -> bool {
        self.real_space_transform_version != self.globe_space_transform_version
    }

    // TODO: document responsibilities of caller.
    // TODO: return translation and orientation.
    pub fn get_real_transform_and_mark_as_clean(&mut self) -> Pt3 {
        self.real_space_transform_version = self.globe_space_transform_version;
        self.real_pos()
    }
}

impl specs::Component for CellDweller {
    type Storage = specs::HashMapStorage<CellDweller>;
}
