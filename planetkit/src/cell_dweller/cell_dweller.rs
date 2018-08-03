use specs;

use globe::Spec;
use grid::{Dir, GridPoint3};
use movement::*;
use types::*;

pub struct CellDweller {
    // TODO: make these private and use guts trait pattern to expose them internally.
    // TODO: is this guts pattern worth a separate macro crate of its own?
    pub pos: GridPoint3,
    pub dir: Dir,
    pub last_turn_bias: TurnDir,
    // Most `CellDweller`s will also be `Spatial`s. Track whether the
    // computed real-space transform has been updated since the globe-space
    // transform was modified so we know when the former is dirty.
    is_real_space_transform_dirty: bool,
    pub globe_spec: Spec,
    pub seconds_between_moves: TimeDelta,
    pub seconds_until_next_move: TimeDelta,
    pub seconds_between_turns: TimeDelta,
    pub seconds_until_next_turn: TimeDelta,
    pub seconds_until_next_fall: TimeDelta,
    pub globe_entity: Option<specs::Entity>,
}

impl CellDweller {
    pub fn new(
        pos: GridPoint3,
        dir: Dir,
        globe_spec: Spec,
        globe_entity: Option<specs::Entity>,
    ) -> CellDweller {
        CellDweller {
            pos: pos,
            dir: dir,
            last_turn_bias: TurnDir::Right,
            is_real_space_transform_dirty: true,
            globe_spec: globe_spec,
            // TODO: accept as parameter
            seconds_between_moves: 0.1,
            seconds_until_next_move: 0.0,
            // TODO: accept as parameter
            seconds_between_turns: 0.2,
            seconds_until_next_turn: 0.0,
            seconds_until_next_fall: 0.0,
            globe_entity: globe_entity,
        }
    }

    pub fn pos(&self) -> GridPoint3 {
        self.pos
    }

    pub fn set_grid_point(&mut self, new_pos: GridPoint3) {
        self.pos = new_pos;
        self.is_real_space_transform_dirty = true;
    }

    pub fn set_cell_transform(
        &mut self,
        new_pos: GridPoint3,
        new_dir: Dir,
        new_last_turn_bias: TurnDir,
    ) {
        self.pos = new_pos;
        self.dir = new_dir;
        self.last_turn_bias = new_last_turn_bias;
        self.is_real_space_transform_dirty = true;
    }

    pub fn dir(&self) -> Dir {
        self.dir
    }

    pub fn turn(&mut self, turn_dir: TurnDir) {
        turn_by_one_hex_edge(
            &mut self.pos,
            &mut self.dir,
            self.globe_spec.root_resolution,
            turn_dir,
        ).expect("This suggests a bug in `movement` code.");
        self.is_real_space_transform_dirty = true;
    }

    /// Calculate position in real-space.
    fn real_pos(&self) -> Pt3 {
        self.globe_spec.cell_bottom_center(self.pos)
    }

    fn real_transform(&self) -> Iso3 {
        let eye = self.real_pos();
        // Look one cell ahead.
        let next_pos = adjacent_pos_in_dir(self.pos, self.dir).unwrap();
        let target = self.globe_spec.cell_bottom_center(next_pos);
        // Calculate up vector. Nalgebra will normalise this so we can
        // just use the eye position as a vector; it points up out from
        // the center of the world already!
        let up = eye.coords;
        Iso3::new_observer_frame(&eye, &target, &up)
    }

    // Make it a long cumbersome name so you make it explicit you're
    // not storing the result on a Spatial.
    pub fn real_transform_without_setting_clean(&self) -> Iso3 {
        self.real_transform()
    }

    pub fn is_real_space_transform_dirty(&self) -> bool {
        self.is_real_space_transform_dirty
    }

    // TODO: document responsibilities of caller.
    // TODO: return translation and orientation.
    pub fn get_real_transform_and_mark_as_clean(&mut self) -> Iso3 {
        self.is_real_space_transform_dirty = false;
        self.real_transform()
    }
}

impl specs::Component for CellDweller {
    type Storage = specs::HashMapStorage<CellDweller>;
}
