// The key perspectives that determined how all this works were roughly:
//
// - Pentagons don't appear to have any neat natural orientation on the globe.
// - There are _lots_ of special cases to take them into account in all their positions
//   if they are treated separately when moving between root quads.
// - There are also several special cases just for normal movement between roots;
//   i.e. even when no pentagons involved.
//
// Therefore, I've decided to reduce the number of special cases
// in the logic by complicating the simple case.
//
// Instead of handling all the different interfaces between different root quads separately,
// movement is broken down into a handful of steps:
//
// - Look up a static map of triangles that are equivalent to the arctic triangle
//   in root 0 to find the closest one that `pos` is in.
// - Transform `pos` and `dir` to be relative to the identified triangle.
//   triangle of root 0.
// - Perform movement and handle special cases of moving east by one root,
//   moving through the pentagon in different ways, etc.
//
// Here's a diagram showing the index of each of the triangles we'll
// use for this:
//
//                  ●
//                 / \
//                / 0 \
//               /     \
//              /       \
//             / 1     2 \
//            ●-----------●
//             \ 5     4 / \
//              \       / 6 \
//               \     /     \
//                \ 3 /       \
//                 \ / 7     8 \
//                  ●-----------●
//                   \ 11   10 /
//                    \       /
//                     \     /
//                      \ 9 /
//                       \ /
//                        ●
//
// This may all look pretty hairy (and it is) but it's also fairly
// easy to thoroughly test; see `test.rs`.

use crate::grid::{DirIndex, GridCoord, RootIndex};

pub struct Triangle {
    // Position in root in terms of x-resolutions.
    pub apex: [GridCoord; 2],
    // Direction of x-axis.
    // TODO: also redundantly store x and y vectors
    // to avoid excessive computation at run time?
    // Only after all tests are passing for slow version!
    pub x_dir: DirIndex,
    // Travelling anti-clockwise around triangle apex,
    // starting with this same triangle.
    pub exits: [Exit; 5],
}
pub struct Exit {
    pub triangle_index: usize,
    pub root_offset: RootIndex,
}

pub static TRIANGLES: [Triangle; 12] = [
    // 0
    Triangle {
        apex: [0, 0],
        x_dir: 0,
        exits: [
            Exit {
                triangle_index: 0,
                root_offset: 0,
            },
            Exit {
                triangle_index: 0,
                root_offset: 1,
            },
            Exit {
                triangle_index: 0,
                root_offset: 2,
            },
            Exit {
                triangle_index: 0,
                root_offset: 3,
            },
            Exit {
                triangle_index: 0,
                root_offset: 4,
            },
        ],
    },
    // 1
    Triangle {
        apex: [1, 0],
        x_dir: 4,
        exits: [
            Exit {
                triangle_index: 1,
                root_offset: 0,
            },
            Exit {
                triangle_index: 2,
                root_offset: 4,
            },
            Exit {
                triangle_index: 4,
                root_offset: 4,
            },
            Exit {
                triangle_index: 6,
                root_offset: 4,
            },
            Exit {
                triangle_index: 5,
                root_offset: 0,
            },
        ],
    },
    // 2
    Triangle {
        apex: [0, 1],
        x_dir: 8,
        exits: [
            Exit {
                triangle_index: 2,
                root_offset: 0,
            },
            Exit {
                triangle_index: 4,
                root_offset: 0,
            },
            Exit {
                triangle_index: 6,
                root_offset: 0,
            },
            Exit {
                triangle_index: 5,
                root_offset: 1,
            },
            Exit {
                triangle_index: 1,
                root_offset: 1,
            },
        ],
    },
    // 3
    Triangle {
        apex: [1, 1],
        x_dir: 6,
        exits: [
            Exit {
                triangle_index: 3,
                root_offset: 0,
            },
            Exit {
                triangle_index: 8,
                root_offset: 4,
            },
            Exit {
                triangle_index: 10,
                root_offset: 4,
            },
            Exit {
                triangle_index: 11,
                root_offset: 0,
            },
            Exit {
                triangle_index: 7,
                root_offset: 0,
            },
        ],
    },
    // 4
    Triangle {
        apex: [0, 1],
        x_dir: 10,
        exits: [
            Exit {
                triangle_index: 4,
                root_offset: 0,
            },
            Exit {
                triangle_index: 6,
                root_offset: 0,
            },
            Exit {
                triangle_index: 5,
                root_offset: 1,
            },
            Exit {
                triangle_index: 1,
                root_offset: 1,
            },
            Exit {
                triangle_index: 2,
                root_offset: 0,
            },
        ],
    },
    // 5
    Triangle {
        apex: [1, 0],
        x_dir: 2,
        exits: [
            Exit {
                triangle_index: 5,
                root_offset: 0,
            },
            Exit {
                triangle_index: 1,
                root_offset: 0,
            },
            Exit {
                triangle_index: 2,
                root_offset: 4,
            },
            Exit {
                triangle_index: 4,
                root_offset: 4,
            },
            Exit {
                triangle_index: 6,
                root_offset: 4,
            },
        ],
    },
    // 6
    Triangle {
        apex: [0, 1],
        x_dir: 0,
        exits: [
            Exit {
                triangle_index: 6,
                root_offset: 0,
            },
            Exit {
                triangle_index: 5,
                root_offset: 1,
            },
            Exit {
                triangle_index: 1,
                root_offset: 1,
            },
            Exit {
                triangle_index: 2,
                root_offset: 0,
            },
            Exit {
                triangle_index: 4,
                root_offset: 0,
            },
        ],
    },
    // 7
    Triangle {
        apex: [1, 1],
        x_dir: 4,
        exits: [
            Exit {
                triangle_index: 7,
                root_offset: 0,
            },
            Exit {
                triangle_index: 3,
                root_offset: 0,
            },
            Exit {
                triangle_index: 8,
                root_offset: 4,
            },
            Exit {
                triangle_index: 10,
                root_offset: 4,
            },
            Exit {
                triangle_index: 11,
                root_offset: 0,
            },
        ],
    },
    // 8
    Triangle {
        apex: [0, 2],
        x_dir: 8,
        exits: [
            Exit {
                triangle_index: 8,
                root_offset: 0,
            },
            Exit {
                triangle_index: 10,
                root_offset: 0,
            },
            Exit {
                triangle_index: 11,
                root_offset: 1,
            },
            Exit {
                triangle_index: 7,
                root_offset: 1,
            },
            Exit {
                triangle_index: 3,
                root_offset: 1,
            },
        ],
    },
    // 9
    Triangle {
        apex: [1, 2],
        x_dir: 6,
        exits: [
            Exit {
                triangle_index: 9,
                root_offset: 0,
            },
            Exit {
                triangle_index: 9,
                root_offset: 4,
            },
            Exit {
                triangle_index: 9,
                root_offset: 3,
            },
            Exit {
                triangle_index: 9,
                root_offset: 2,
            },
            Exit {
                triangle_index: 9,
                root_offset: 1,
            },
        ],
    },
    // 10
    Triangle {
        apex: [0, 2],
        x_dir: 10,
        exits: [
            Exit {
                triangle_index: 10,
                root_offset: 0,
            },
            Exit {
                triangle_index: 11,
                root_offset: 1,
            },
            Exit {
                triangle_index: 7,
                root_offset: 1,
            },
            Exit {
                triangle_index: 3,
                root_offset: 1,
            },
            Exit {
                triangle_index: 8,
                root_offset: 0,
            },
        ],
    },
    // 11
    Triangle {
        apex: [1, 1],
        x_dir: 2,
        exits: [
            Exit {
                triangle_index: 11,
                root_offset: 0,
            },
            Exit {
                triangle_index: 7,
                root_offset: 0,
            },
            Exit {
                triangle_index: 3,
                root_offset: 0,
            },
            Exit {
                triangle_index: 8,
                root_offset: 4,
            },
            Exit {
                triangle_index: 10,
                root_offset: 4,
            },
        ],
    },
];
