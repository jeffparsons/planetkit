pub type DirIndex = u8;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Dir {
    pub index: DirIndex,
}

impl Dir {
    pub fn new(index: DirIndex) -> Dir {
        Dir {
            index: index
        }
    }
}

impl From<DirIndex> for Dir {
    fn from(dir_index: DirIndex) -> Dir {
        Dir::new(dir_index)
    }
}
