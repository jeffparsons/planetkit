// Don't make `globe` public; we re-export the
// main bits at this level below.
mod globe;
pub mod icosahedron;
mod spec;
pub mod chunk;
mod root;
pub mod cell_shape;
mod view;
mod gen;
mod cell_pos;
mod neighbors;
mod dir;
mod chunk_view;
mod chunk_view_system;
mod cursor;

#[cfg(test)]
mod tests;

use types::*;

// TODO: be selective in what you export; no wildcards!
pub use self::root::*;
pub use self::globe::Globe;
pub use self::spec::*;
pub use self::view::*;
pub use self::cell_pos::*;
pub use self::neighbors::*;
pub use self::dir::*;
pub use self::chunk_view::*;
pub use self::chunk_view_system::*;
pub use self::cursor::Cursor;

pub type IntCoord = i64;

// TODO: move project into icosahedron module.

// Project a position in a given root quad into a unit sphere.
// Assumes that one corner is represented in `pt_in_root_quad`
// as (0, 0) and the opposite is (1, 1).
#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
pub fn project(root: Root, mut pt_in_root_quad: Pt2) -> Pt3 {
    // An icosahedron can be flattened into a net comprising 20 triangles:
    //
    //      ●     ●     ●     ●     ●
    //     / \   / \   / \   / \   / \
    //    /   \ /   \ /   \ /   \ /   \
    //   ●-----●-----●-----●-----●-----●
    //    \   / \   / \   / \   / \   / \
    //     \ /   \ /   \ /   \ /   \ /   \
    //      ●-----●-----●-----●-----●-----●
    //       \   / \   / \   / \   / \   /
    //        \ /   \ /   \ /   \ /   \ /
    //         ●     ●     ●     ●     ●
    //
    // We can then break this into 5 "root quads", each of which comprises
    // a strip of four triangles running all the way from the north pole
    // of the globe down to the south. These root quads are twice as long
    // (y-axis) as they are wide (x-axis).
    //
    // Highlighting the first quad:
    //
    //      ●     ◌     ◌     ◌     ◌
    //     /·\   / \   / \   / \   / \
    //    /···\ /   \ /   \ /   \ /   \
    //   ● - - ●-----◌-----◌-----◌-----◌
    //    \···/·\   / \   / \   / \   / \
    //     \·/···\ /   \ /   \ /   \ /   \
    //      ● - - ●-----◌-----◌-----◌-----◌
    //       \···/ \   / \   / \   / \   /
    //        \·/   \ /   \ /   \ /   \ /
    //         ●     ◌     ◌     ◌     ◌
    //
    // So we need to set up some points and vectors based on the
    // root we're operating in, and then the math depends on which
    // triangle `pt_in_root_quad` is in.
    //
    // In the diagram below, each point is labelled with this information
    // in the same order:
    //
    //   - A name for this vertex (e.g. 'a') for use in calculations below
    //   - The (x, y) coordinates for this vertex in voxmap space
    //   - The indexes of the triangle, and the index of the vertex within
    //     each triangle, expressed as `i_j`. For many vertices there will
    //     be multiple triangles containing it.
    //
    //                   a (0, 0)
    //                     0_0
    //                 ●
    //                / \
    //               /   \
    //     b        /  0  \     c (0, 1)
    //   (1, 0)    /       \      0_2
    //    0_1     ●---------●     1_1
    //    1_2      \       / \    2_0
    //              \  1  /   \
    //               \   /  2  \
    //         d      \ /       \
    //       (1, 1)    ●---------●    e (0, 2)
    //         1_0      \       /       2_2
    //         2_1       \  3  /        3_1
    //         3_2        \   /
    //                     \ /
    //               f      ●
    //             (1, 2)
    //              3_0
    //
    // TODO: cache all this stuff somewhere. It's tiny, and we'll use it heaps.
    use self::icosahedron::{ FACES, VERTICES };
    let triangle_indices = [
        root.index as usize * 4,
        root.index as usize * 4 + 1,
        root.index as usize * 4 + 2,
        root.index as usize * 4 + 3,
    ];
    let faces = [
        FACES[triangle_indices[0]],
        FACES[triangle_indices[1]],
        FACES[triangle_indices[2]],
        FACES[triangle_indices[3]],
    ];
    let a: Pt3 = (&VERTICES[faces[0][0]]).into();
    let b: Pt3 = (&VERTICES[faces[0][1]]).into();
    let c: Pt3 = (&VERTICES[faces[1][1]]).into();
    let d: Pt3 = (&VERTICES[faces[1][0]]).into();
    let e: Pt3 = (&VERTICES[faces[3][1]]).into();
    let f: Pt3 = (&VERTICES[faces[3][0]]).into();

    // Triangle 0
    let ab = b - a;
    let ac = c - a;
    // Triangle 1
    let db = b - d;
    let dc = c - d;
    // Triangle 2
    let cd = d - c;
    let ce = e - c;
    // Triangle 3
    let fd = d - f;
    let fe = e - f;

    // It'll be easier to do the math we need here if the positions
    // lie between (0, 0) and (1, 2).
    pt_in_root_quad[1] *= 2.0;

    // Decide which triangle we're in.
    let pos_on_icosahedron = if pt_in_root_quad[0] + pt_in_root_quad[1] < 1.0 {
        // In triangle 0.
        a + ab * pt_in_root_quad[0] + ac * pt_in_root_quad[1]
    } else if pt_in_root_quad[1] < 1.0 {
        // In triangle 1.
        d + dc * (1.0 - pt_in_root_quad[0]) + db * (1.0 - pt_in_root_quad[1])
    } else if pt_in_root_quad[0] + pt_in_root_quad[1] < 2.0 {
        // In triangle 2.
        // Bring the y-value back into [0, 1] so we can just repeat the math from above.
        pt_in_root_quad[1] -= 1.0;
        c + cd * pt_in_root_quad[0] + ce * pt_in_root_quad[1]
    } else {
        // In triangle 3.
        // Bring the y-value back into [0, 1] so we can just repeat the math from above.
        pt_in_root_quad[1] -= 1.0;
        f + fe * (1.0 - pt_in_root_quad[0]) + fd * (1.0 - pt_in_root_quad[1])
    };
    use na::Norm;
    *pos_on_icosahedron.as_vector().normalize().as_point()
}

/// Calculate the origin of a chunk that contains the given `pos`,
/// with the guarantee that the chunk will be in the same root even
/// if `pos` is on the edge of that root.
///
/// Note that this pays no attention to what chunk _owns_ the cell,
/// so you should assume that any chunk in this root that contains
/// the position at all may be returned.
pub fn origin_of_chunk_in_same_root_containing(
    pos: CellPos,
    root_resolution: [IntCoord; 2],
    chunk_resolution: [IntCoord; 3],
) -> ChunkOrigin {
    // Calculate x-position of a containing chunk.
    let end_x = root_resolution[0];
    let chunk_origin_x = if pos.x == end_x {
        // Instead of trying to find a chunk beyond those that exist,
        // just use the last chunk in the x-direction; `pos` is in that.
        (end_x / chunk_resolution[0] - 1) * chunk_resolution[0]
    } else {
        pos.x / chunk_resolution[0] * chunk_resolution[0]
    };

    // Calculate y-position of a containing chunk.
    let end_y = root_resolution[1];
    let chunk_origin_y = if pos.y == end_y {
        // Instead of trying to find a chunk beyond those that exist,
        // just use the last chunk in the y-direction; `pos` is in that.
        (end_y / chunk_resolution[1] - 1) * chunk_resolution[1]
    } else {
        pos.y / chunk_resolution[1] * chunk_resolution[1]
    };

    // Z-position is easy; there's no sharing of cells on the z-axis.
    let chunk_origin_z = pos.z / chunk_resolution[2] * chunk_resolution[2];

    ChunkOrigin::new(
        CellPos {
            root: pos.root,
            x: chunk_origin_x,
            y: chunk_origin_y,
            z: chunk_origin_z,
        },
        root_resolution,
        chunk_resolution,
    )
}

pub fn origin_of_chunk_owning(
    pos_in_owning_root: PosInOwningRoot,
    root_resolution: [IntCoord; 2],
    chunk_resolution: [IntCoord; 3],
) -> ChunkOrigin {
    let pos: CellPos = pos_in_owning_root.into();

    // Figure out what chunk this is in.
    let end_x = root_resolution[0];
    let end_y = root_resolution[1];
    let last_chunk_x = (end_x / chunk_resolution[0] - 1) * chunk_resolution[0];
    let last_chunk_y = (end_y / chunk_resolution[1] - 1) * chunk_resolution[1];
    // Cells aren't shared by chunks in the z-direction, so the z-origin
    // is the same across all cases. Small mercies.
    let chunk_origin_z = pos.z / chunk_resolution[2] * chunk_resolution[2];
    if pos.x == 0 && pos.y == 0 {
        // Chunk at (0, 0) owns north pole.
        ChunkOrigin::new(
            CellPos {
                root: pos.root,
                x: 0,
                y: 0,
                z: chunk_origin_z,
            },
            root_resolution,
            chunk_resolution,
        )
    } else if pos.x == end_x && pos.y == end_y {
        // Chunk at (last_chunk_x, last_chunk_y) owns south pole.
        ChunkOrigin::new(
            CellPos {
                root: pos.root,
                x: last_chunk_x,
                y: last_chunk_y,
                z: chunk_origin_z,
            },
            root_resolution,
            chunk_resolution,
        )
    } else {
        // Chunks own cells on their edge at `local_x == 0`, and their edge at `local_y == chunk_resolution`.
        // The cells on other edges belong to adjacent chunks.
        let chunk_origin_x = pos.x / chunk_resolution[0] * chunk_resolution[0];
        // Shift everything down by one in y-direction.
        let chunk_origin_y = (pos.y - 1) / chunk_resolution[1] * chunk_resolution[1];
        ChunkOrigin::new(
            CellPos {
                root: pos.root,
                x: chunk_origin_x,
                y: chunk_origin_y,
                z: chunk_origin_z,
            },
            root_resolution,
            chunk_resolution,
        )
    }
}
