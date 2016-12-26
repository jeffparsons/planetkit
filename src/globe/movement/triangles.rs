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

use na;

use super::super::{ IntCoord, DirIndex };

type Pos2 = na::Point2<IntCoord>;

pub struct Triangle {
    // Position in root in terms of x-resolutions.
    pub apex: Pos2,
    // Direction of x-axis.
    // TODO: also redundantly store x and y vectors
    // to avoid excessive computation at run time?
    // Only after all tests are passing for slow version!
    pub x_dir: DirIndex,
    // Travelling anti-clockwise around triangle apex.
    pub exits: [usize; 4],
}

pub const TRIANGLES: [Triangle; 12] = [
    // 0
    Triangle {
        apex: Pos2 { x: 0, y: 0 },
        x_dir: 0,
        exits: [ 0, 0, 0, 0 ],
    },

    // TODO:
    // 1
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 2
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 3
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 4
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 5
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 6
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 7
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 8
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },

    // 9
    Triangle {
        apex: Pos2 { x: 1, y: 2 },
        x_dir: 6,
        exits: [ 9, 9, 9, 9 ],
    },

    // TODO:
    // 10
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
    // 11
    Triangle { apex: Pos2 { x: 0, y: 0 }, x_dir: 0, exits: [ 0, 0, 0, 0 ], },
];
