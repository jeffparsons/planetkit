pub type RootIndex = u8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Root {
    pub index: RootIndex,
}

impl Root {
    pub fn new(index: RootIndex) -> Root {
        Root {
            index: index
        }
    }

    pub fn next_west(&self) -> Root {
        Root {
            index: ((self.index + (5 - 1)) % 5),
        }
    }
}

// TODO: we'll probably want to make this panic if you enter something
// out of bounds, so this implementation is probably illegal. (IIRC `from` should not panic.)
impl From<RootIndex> for Root {
    fn from(root_index: RootIndex) -> Root {
        Root::new(root_index)
    }
}

#[cfg(test)]
mod test {
    use super::Root;

    #[test]
    fn next_west() {
        let root: Root = 3.into();
        assert_eq!(2, root.next_west().index);
    }
}
