// Don't make this public; we re-export everything
// from it below.
mod globe;
pub mod icosahedron;
pub mod chunk;

// Re-export everything from `globe` module directly
// at this level. It's the main subject of this module,
// so we're really only putting it in a module of its
// own for source organisation.
pub use self::globe::*;
