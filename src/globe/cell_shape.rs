//! Facts about the geometry and topology of hexagons, and portions thereof,
//! with respect to their use in a globe voxmap. These hexagonal portions
//! are analogous to circular sectors, and are used in rendering the edges and
//! corners of chunks where a full hexagon will not fit.

use super::IntCoord;

// We can imagine a hexagon laid out on a quad
// that wraps in both directions, such that its
// center exists at all four corners of the quad:
//
//  (0, 0)
//          ●       y
//    x        ◌      ↘
//    ↓     ◌     ◌
//             ◌     ●
//          ◌     ◌ /   ◌
//             ◌   / ◌     ◌     (0, 0)
//          ●-----●     ◌     ●
//             ◌   \ ◌     ◌
//          ◌     ◌ \   ◌     ◌
//             ◌     ●     ◌
//          ◌     ◌   \ ◌     ◌
//             ◌     ◌ \   ◌
//          ●     ◌     ●-----●
//  (0, 0)     ◌     ◌ /   ◌
//                ◌   / ◌     ◌
//                   ●     ◌
//                      ◌     ◌
//                         ◌
//                            ●
//                               (0, 0)
//
// This makes it visually obvious that we're dealing
// with a grid of 6 units between hexagon centers (count it)
// to calculate cell vertex positions (if we want all vertices
// to lie at integer coordinate pairs) as opposed to the 1 unit
// between cell centers when we're only concerned with the
// center points of each cell.
//
// Then, if we list out points for the middle of each side and each vertex,
// starting from the middle of the side in the positive x direction and
// travelling counterclockwise, we end up with 12 offset coordinate pairs
// in this grid, labelled as follows:
//
//                 6
//        7                 5
//           ●-----●-----●
//          /   ◌     ◌   \
//         / ◌     ◌     ◌ \
//     8  ●     ◌     ◌     ●  4
//       /   ◌     ◌     ◌   \
//      / ◌     ◌     ◌     ◌ \
//  9  ●     ◌     ●     ◌     ●  3
//      \ ◌     ◌     ◌     ◌ /
//       \   ◌     ◌     ◌   /
//        ●     ◌     ◌     ●
//    10   \ ◌     ◌     ◌ /   2
//          \   ◌     ◌   /         y
//           ●-----●-----●           ↘
//       11                 1
//                 0
//
//                 x
//                 ↓
//
// Referring to the top figure for the offsets and the
// bottom for the labelling, that gives us:
pub const DIR_OFFSETS: [[i64; 2]; 12] = [
    [ 3,  0], // edge (+x)
    [ 2,  2], // vertex
    [ 0,  3], // edge (+y)
    [-2,  4], // vertex
    [-3,  3], // edge
    [-4,  2], // vertex
    [-3,  0], // edge (-x)
    [-2, -2], // vertex
    [ 0, -3], // edge (-y)
    [ 2, -4], // vertex
    [ 3, -3], // edge
    [ 4, -2], // vertex
];

// There are 9 different shapes that we need for drawing cells in various
// parts of a chunk. Here is a cross-section of the smallest chunk that
// demonstrates all 9 shapes:
//
//       ●       y
//   x   |`~◌      ↘
//   ↓   ◌   `~◌
//       |  ◌   `~●
//       ◌     ◌ / `~◌
//       |  ◌   / ◌   `~◌
//       ●-----●     ◌   `~◌
//       |  ◌   \ ◌     ◌   `~◌
//       ◌     ◌ \   ◌     ◌   `~◌
//       |  ◌     ◌     ◌     ◌   `~●
//       ◌     ◌   \ ◌     ◌     ◌ / `~◌
//       |  ◌     ◌ \   ◌     ◌   / ◌   `~◌
//       ◌     ◌     ●-----◌-----●     ◌   `~●
//       |  ◌     ◌ /   ◌     ◌   \ ◌     ◌  |
//       ◌     ◌   / ◌     ◌     ◌ \   ◌     ◌
//       |  ◌     ◌     ◌     ◌     ◌     ◌  |
//       ◌     ◌ /   ◌     ◌     ◌   \ ◌     ◌
//       |  ◌   /       ◌     ◌     ◌ \   ◌  |
//       ●-----●     ◌     ◌     ◌     ●-----●
//       |  ◌   \ ◌     ◌     ◌     ◌ /   ◌  |
//       ◌     ◌ \   ◌     ◌     ◌   / ◌     ◌
//       |  ◌     ◌     ◌     ◌     ◌     ◌  |
//       ◌     ◌   \ ◌     ◌     ◌ /   ◌     ◌
//       |  ◌     ◌ \   ◌     ◌   / ◌     ◌  |
//       ●     ◌     ●-----◌-----●     ◌     ◌
//        `~◌     ◌ /   ◌     ◌   \ ◌     ◌  |
//           `~◌   / ◌     ◌     ◌ \   ◌     ◌
//              `~●     ◌     ◌     ◌     ◌  |
//                 `~◌     ◌     ◌   \ ◌     ◌
//                    `~◌     ◌     ◌ \   ◌  |
//                       `~◌     ◌     ●-----●
//                          `~◌     ◌ /   ◌  |
//                             `~◌   / ◌     ◌
//                                `~●     ◌  |
//                                   `~◌     ◌
//                                      `~◌  |
//                                         `~●
//
// The filled circles represent vertices that will be used in the
// geometry for a given shape. Note that cell centres are included
// explicitly only where needed.
//
// For consistency with the orientation of chunks on a globe,
// we will refer to the corner at (0, 0) as "North".

pub struct CellShape {
    pub top_outline_dir_offsets: &'static [[i64; 2]],
}

pub const FULL_HEX: CellShape = CellShape {
    top_outline_dir_offsets: &[
        DIR_OFFSETS[1],
        DIR_OFFSETS[3],
        DIR_OFFSETS[5],
        DIR_OFFSETS[7],
        DIR_OFFSETS[9],
        DIR_OFFSETS[11],
    ],
};

pub const NORTH_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        [0, 0],
        DIR_OFFSETS[0],
        DIR_OFFSETS[1],
        DIR_OFFSETS[2],
    ],
};

pub const SOUTH_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        [0, 0],
        DIR_OFFSETS[6],
        DIR_OFFSETS[7],
        DIR_OFFSETS[8],
    ],
};

pub const WEST_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        [0, 0],
        DIR_OFFSETS[2],
        DIR_OFFSETS[3],
        DIR_OFFSETS[5],
        DIR_OFFSETS[6],
    ],
};

pub const EAST_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        [0, 0],
        DIR_OFFSETS[8],
        DIR_OFFSETS[9],
        DIR_OFFSETS[11],
        DIR_OFFSETS[0],
    ],
};

pub const NORTH_WEST_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        DIR_OFFSETS[0],
        DIR_OFFSETS[1],
        DIR_OFFSETS[3],
        DIR_OFFSETS[5],
        DIR_OFFSETS[6],
    ],
};

pub const NORTH_EAST_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        DIR_OFFSETS[8],
        DIR_OFFSETS[9],
        DIR_OFFSETS[11],
        DIR_OFFSETS[1],
        DIR_OFFSETS[2],
    ],
};

pub const SOUTH_WEST_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        DIR_OFFSETS[2],
        DIR_OFFSETS[3],
        DIR_OFFSETS[5],
        DIR_OFFSETS[7],
        DIR_OFFSETS[8],
    ],
};

pub const SOUTH_EAST_PORTION: CellShape = CellShape {
    top_outline_dir_offsets: &[
        DIR_OFFSETS[0],
        DIR_OFFSETS[6],
        DIR_OFFSETS[7],
        DIR_OFFSETS[9],
        DIR_OFFSETS[11],
    ],
};

// If we number hexagonal cell edges from 0 through 5,
// then the (x, y) offsets to reach each neighbouring hexagon are:
//
//           (-1, 0)
//        \     3     /
//         \         /
// (0, -1)  ●-------●      (-1, +1)
//    4    /         \   2
//        /           \
//       /             \
// -----●       ◌       ●-----
//       \             /
//        \           /
//     5   \         /   1
// (+1, -1) ●-------●      (0, +1)
//         /         \
//        /     0     \
//           (+1, 0)
//
//              x
//              ↓
pub static NEIGHBOR_OFFSETS: [(IntCoord, IntCoord); 6] = [
    (  1,  0 ),
    (  0,  1 ),
    ( -1,  1 ),
    ( -1,  0 ),
    (  0, -1 ),
    (  1, -1 ),
];
