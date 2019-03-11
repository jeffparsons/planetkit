use super::Point3;
use super::{GridCoord, Root, RootIndex};

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct Point2 {
    pub root: Root,
    pub x: GridCoord,
    pub y: GridCoord,
}

impl Point2 {
    pub fn new(root: Root, x: GridCoord, y: GridCoord) -> Point2 {
        Point2 { root, x, y }
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

    pub fn with_z(&self, z: GridCoord) -> Point3 {
        Point3::new(self.root, self.x, self.y, z)
    }
}
