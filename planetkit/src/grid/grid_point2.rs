use super::{ IntCoord, Root, RootIndex };
use super::CellPos;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GridPoint2 {
    pub root: Root,
    pub x: IntCoord,
    pub y: IntCoord,
}

impl GridPoint2 {
    pub fn new(root: Root, x: IntCoord, y: IntCoord) -> GridPoint2 {
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

    pub fn with_x(&self, new_x: IntCoord) -> Self {
        let mut new_point = *self;
        new_point.x = new_x;
        new_point
    }

    pub fn with_y(&self, new_y: IntCoord) -> Self {
        let mut new_point = *self;
        new_point.y = new_y;
        new_point
    }

    pub fn with_z(&self, z: IntCoord) -> CellPos {
        CellPos::new(self.root, self.x, self.y, z)
    }
}
