// Golden ratio
#[allow(clippy::unreadable_literal, clippy::excessive_precision)]
const PHI: f64 = 1.61803398874989484820458683436563811772030917980576286213544862270526046281890244970720720418939113748475;

// Scale factor to get icosahedron with circumscribed sphere of radius 1
#[allow(clippy::unreadable_literal, clippy::excessive_precision)]
const SF: f64 = 0.525731112119133606025669084847876607285497932243341781528935523241211146403214018371632628831552570956698521400021;

// Vertices of an icosahedron with circumscribed sphere of radius 1
// REVISIT: consider precalculating these numbers instead of basing them on the values above.
const A: f64 = SF;
const B: f64 = PHI * SF;
#[rustfmt::skip]
pub const VERTICES: [[f64; 3]; 12] = [
    [ 0.0,  A,    B  ],
    [ 0.0, -A,    B  ],
    [ 0.0, -A,   -B  ],
    [ 0.0,  A,   -B  ],
    [ A,    B,    0.0],
    [-A,    B,    0.0],
    [-A,   -B,    0.0],
    [ A,   -B,    0.0],
    [ B,    0.0,  A  ],
    [-B,    0.0,  A  ],
    [-B,    0.0, -A  ],
    [ B,    0.0, -A  ],
];

// TODO: describe the very specific and deliberate
// order that these faces are in.
#[rustfmt::skip]
pub const FACES: [[usize; 3]; 20] = [
    [ 0,  1,  8  ],
    [ 7,  8,  1  ],
    [ 8,  7,  11 ],
    [ 2,  11, 7  ],

    [ 0,  8,  4  ],
    [ 11, 4,  8  ],
    [ 4,  11, 3  ],
    [ 2,  3,  11 ],

    [ 0,  4,  5  ],
    [ 3,  5,  4  ],
    [ 5,  3,  10 ],
    [ 2,  10, 3  ],

    [ 0,  5,  9  ],
    [ 10, 9,  5  ],
    [ 9,  10, 6  ],
    [ 2,  6,  10 ],

    [ 0,  9,  1  ],
    [ 6,  1,  9  ],
    [ 1,  6,  7  ],
    [ 2,  7,  6  ],
];
