// Don't make `globe` public; we re-export the
// main bits at this level below.
mod globe;
mod icosahedron;
mod spec;
pub mod chunk;
mod root;

use types::*;

pub use self::root::*;
pub use self::globe::*;
pub use self::spec::*;

pub type IntCoord = u64;

pub struct Dir(u8);

// Project a position in a given root quad into a unit sphere.
// Assumes that one corner is represented in `pt_in_root_quad`
// as (0, 0) and the opposite is (1, 1).
pub fn project(root: Root, pt_in_root_quad: Pt2) -> Pt3 {
    // Each root quad comprises two triangles of the icosahedron.
    // So we need to set up some points and vectors based on the
    // root we're operating in, and then the math depends on which
    // triangle `pt_in_root_quad` is in.
    //
    // See the diagram below for a visual description of how 2-space
    // coordinates on the quad relate to the the icoshaedral vertices.
    // `N` and `S` refer to the north and south triangles respectively.
    //
    //     a    ________  c (0, 1)
    //  (0, 0)  \      /\    N_2
    //    N_0    \ N  /  \   S_1
    //            \  /  S \
    //             \/______\
    //                       d (1, 1)
    //            b (1, 0)     S_0
    //              N_1
    //              S_2
    //
    // TODO: cache all this stuff somewhere. It's tiny, and we'll use it heaps.
    use self::icosahedron::{ FACES, VERTICES };
    let i_north = root.index as usize * 2;
    let i_south = i_north + 1;
    let north = FACES[i_north];
    let south = FACES[i_south];
    let a: Pt3 = (&VERTICES[north[0]]).into();
    let b: Pt3 = (&VERTICES[north[1]]).into();
    let c: Pt3 = (&VERTICES[north[2]]).into();
    let d: Pt3 = (&VERTICES[south[0]]).into();

    let ab = b - a;
    let ac = c - a;
    let db = b - d;
    let dc = c - d;

    // Decide which triangle we're in.
    let pos_on_icosahedron = if pt_in_root_quad[0] + pt_in_root_quad[1] < 1.0 {
        // In first triangle.
        a + ab * pt_in_root_quad[0] + ac * pt_in_root_quad[1]
    } else {
        // In second triangle.
        d + dc * (1.0 - pt_in_root_quad[0]) + db * (1.0 - pt_in_root_quad[1])
    };
    use na::Norm;
    *pos_on_icosahedron.as_vector().normalize().as_point()
}