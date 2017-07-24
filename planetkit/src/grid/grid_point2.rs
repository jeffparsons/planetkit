use super::{GridCoord, Root, RootIndex};
use super::GridPoint3;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GridPoint2 {
    pub root: Root,
    pub x: GridCoord,
    pub y: GridCoord,
}

impl GridPoint2 {
    pub fn new(root: Root, x: GridCoord, y: GridCoord) -> GridPoint2 {
        GridPoint2 {
            root: root,
            x: x,
            y: y,
        }
    }

    pub fn with_root(&self, new_root_index: RootIndex) -> Self {
        let mut new_point = *self;
        new_point.root.index = new_root_index;
        new_point
    }

    pub fn with_x(&self, new_x: GridCoord) -> Self {
        let mut new_point = *self;
        new_point.x = new_x;
        new_point
    }

    pub fn with_y(&self, new_y: GridCoord) -> Self {
        let mut new_point = *self;
        new_point.y = new_y;
        new_point
    }

    pub fn with_z(&self, z: GridCoord) -> GridPoint3 {
        GridPoint3::new(self.root, self.x, self.y, z)
    }
}
