use globe::{ IntCoord, Root };

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CellPos {
    pub root: Root,
    pub x: IntCoord,
    pub y: IntCoord,
    pub z: IntCoord,
}
