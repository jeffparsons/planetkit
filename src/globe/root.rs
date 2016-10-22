pub type RootIndex = u8;

#[derive(Clone, Copy)]
pub struct Root {
    pub index: RootIndex,
}

impl Root {
    pub fn new(index: RootIndex) -> Root {
        Root {
            index: index
        }
    }
}

impl From<RootIndex> for Root {
    fn from(root_index: RootIndex) -> Root {
        Root::new(root_index)
    }
}
