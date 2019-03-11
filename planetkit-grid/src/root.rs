pub type RootIndex = u8;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct Root {
    pub index: RootIndex,
}

impl Root {
    pub fn new(index: RootIndex) -> Root {
        Root { index }
    }

    pub fn next_east(self) -> Root {
        Root {
            index: ((self.index + 1) % 5),
        }
    }

    pub fn next_west(self) -> Root {
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

// Occasionally useful around the place when we need to iterate over all roots.
pub static ROOTS: [Root; 5] = [
    Root { index: 0 },
    Root { index: 1 },
    Root { index: 2 },
    Root { index: 3 },
    Root { index: 4 },
];

#[cfg(test)]
mod tests {
    use super::Root;

    #[test]
    fn next_east() {
        let root: Root = 3.into();
        assert_eq!(4, root.next_east().index);

        let root: Root = 4.into();
        assert_eq!(0, root.next_east().index);
    }

    #[test]
    fn next_west() {
        let root: Root = 3.into();
        assert_eq!(2, root.next_west().index);

        let root: Root = 0.into();
        assert_eq!(4, root.next_west().index);
    }
}
