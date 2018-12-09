use super::chunk::{Cell, Chunk};
use super::{ChunkOrigin, Globe};
use crate::globe::globe::GlobeGuts;
use crate::grid::GridPoint3;

// TODO: describe how it only changes between chunks when _necessary_,
// with reference to shared cells. Also remark on the fact that we might
// arbitrarily choose to look for a chunk that does not include the given
// cell even if there is another chunk that does include it but is not loaded.

// TODO: re-introduce original implementation of `Cursor` (non-mutable Globe)
// by specialising `current_chunk` to be `Option<&'a Chunk>` for that one,
// and just a chunk origin for CursorMut?

/// A cell-oriented view into a globe.
///
/// Behaves similarly to an iterator, in that it borrows the globe
/// being viewed, but unlike an iterator it offers multiple methods
/// for navigating between elements.
///
/// `Cursor` exists to help with algorithms that perform a lot of local
/// navigation between cells, by avoiding the high cost of random access.
#[derive(Clone)]
pub struct Cursor<'a> {
    globe: &'a Globe,
    pos: GridPoint3,
    // Avoid needing to find a chunk containing `pos` when moving to another cell,
    // if the last chunk we were working in still contains it.
    //
    // May be `None` if the chunk pointed at is not loaded.
    current_chunk: Option<&'a Chunk>,
    current_chunk_might_be_dirty: bool,
}

/// A cell-oriented view into a mutable globe.
///
/// Behaves similarly to an iterator, in that it borrows the globe
/// being viewed, but unlike an iterator it offers multiple methods
/// for navigating between elements.
///
/// `CursorMut` exists to help with algorithms that perform a lot of local
/// navigation between cells, by avoiding the high cost of random access.
///
/// Mutable cursor is needed for anything that either needs to manipulate
/// the cells in the globe, or anything else on the globe like which chunks
/// are loaded.
pub struct CursorMut<'a> {
    globe: &'a mut Globe,
    pos: GridPoint3,
    // Avoid needing to find current chunk again when moving to another
    // pos if we're moving into another pos in the same chunk.
    //
    // May be `None` if the chunk pointed at is not loaded.
    current_chunk_origin: Option<ChunkOrigin>,
    current_chunk_might_be_dirty: bool,
}

// The shared definition of the `Cursor` and `CursorMut` cursors
macro_rules! cursor {
    ($name:ident, $globe:ty, $cell:ty, $chunks_fn:ident, $chunks_get_fn:ident, $cell_fn:ident) => {
        impl<'a> $name<'a> {
            // NOTE: do not make any function here just called `new`;
            // it's super-easy to misuse `Cursor`s, because a lot of the
            // time you really will mean to read cells from a specific
            // chunk, even if that's not the owner of the cell, or the most
            // obvious chunk when you truncate the position.

            /// Creates a new cursor at the origin of the given chunk.
            ///
            /// Use this if you know that a particular chunk is loaded,
            /// and you want to read cells from that chunk rather than
            /// any neighboring chunk that might share the same cells.
            ///
            /// Note that the given chunk might not _own_ the cells you're
            /// asking for; they might belong to a different root quad.
            pub fn new_in_chunk(globe: $globe, chunk_origin: ChunkOrigin) -> $name<'a> {
                let mut cursor = $name::new_without_chunk_hint(globe, chunk_origin.into());
                cursor.update_current_chunk_origin();
                cursor
            }

            pub fn pos(&self) -> GridPoint3 {
                self.pos
            }

            pub fn set_pos(&mut self, new_pos: GridPoint3) {
                self.pos = new_pos;
                self.current_chunk_might_be_dirty = true;
            }

            pub fn globe(&self) -> &Globe {
                self.globe
            }

            /// Get a reference to the cell the cursor is currently pointing at.
            ///
            /// Note that this cell might come from a chunk that doesn't own it,
            /// unless you've deliberately ensured you pointed at the cell in its
            /// owning root.
            ///
            /// Returns `None` if the requested cell is in a chunk that isn't loaded.
            pub fn cell(&mut self) -> Option<$cell> {
                // Find the owning chunk if necessary.
                let pos = self.pos;
                self.update_current_chunk_origin();
                self.current_chunk().map(|chunk| chunk.$cell_fn(pos))
            }

            // Sets `self.current_chunk_origin` to `None` if the cell pointed
            // at is in a chunk that isn't loaded.
            fn update_current_chunk_origin(&mut self) {
                if !self.current_chunk_might_be_dirty {
                    // Nothing interesting has happened since we last
                    // update the current chunk.
                    return;
                }

                let pos = self.pos;
                let current_chunk_contains_pos = self
                    .current_chunk()
                    .map(|chunk| chunk.contains_pos(pos))
                    .unwrap_or(false);
                if current_chunk_contains_pos {
                    // No need to change chunk; current chunk still contains pos.
                    self.current_chunk_might_be_dirty = false;
                    return;
                }

                // We either have no current chunk or the current pos is no longer
                // within its bounds.
                //
                // Find a chunk that contains pos. Note that it probably won't be
                // the chunk that _owns_ pos; we'll arbitrarily use any chunk in the same
                // root as the given pos that contains pos. This may change in future.
                let chunk_origin = self.globe.origin_of_chunk_in_same_root_containing(self.pos);
                self.set_current_chunk(chunk_origin);
            }
        }
    };
}

// Lifetimes are different because we can't hand out a mutable `Cell` reference
// that outlives self.
cursor! {Cursor, &'a Globe, &'a Cell, chunks, get, cell}
cursor! {CursorMut, &'a mut Globe, &mut Cell, chunks_mut, get_mut, cell_mut}

impl<'a> Cursor<'a> {
    /// Only use this if you don't care about which chunk
    /// we'll attempt to read cells from, keeping in mind that
    /// cells at the edge of chunks are shared between chunks.
    ///
    /// If you know that one particular chunk containing a given
    /// cell is loaded, and you want to read from that chunk, you should
    /// use `new_in_chunk` instead.
    fn new_without_chunk_hint(globe: &'a Globe, pos: GridPoint3) -> Cursor<'a> {
        Cursor {
            globe: globe,
            pos: pos,
            current_chunk: None,
            current_chunk_might_be_dirty: true,
        }
    }

    fn current_chunk(&self) -> Option<&'a Chunk> {
        self.current_chunk
    }

    fn set_current_chunk(&mut self, new_chunk_origin: ChunkOrigin) {
        // Note that this might not be loaded. (`get` might return `None`.)
        self.current_chunk = self.globe.chunks().get(&new_chunk_origin);
        self.current_chunk_might_be_dirty = false;
    }
}

impl<'a> CursorMut<'a> {
    // See `Globe::ensure_chunk_present`.
    pub fn ensure_chunk_present(&mut self) {
        let chunk_origin: ChunkOrigin =
            self.globe.origin_of_chunk_in_same_root_containing(self.pos);
        self.globe.ensure_chunk_present(chunk_origin);
    }

    /// Only use this if you don't care about which chunk
    /// we'll attempt to read cells from, keeping in mind that
    /// cells at the edge of chunks are shared between chunks.
    ///
    /// If you know that one particular chunk containing a given
    /// cell is loaded, and you want to read from that chunk, you should
    /// use `new_in_chunk` instead.
    fn new_without_chunk_hint(globe: &'a mut Globe, pos: GridPoint3) -> CursorMut<'a> {
        CursorMut {
            globe: globe,
            pos: pos,
            current_chunk_origin: None,
            current_chunk_might_be_dirty: true,
        }
    }

    fn current_chunk(&mut self) -> Option<&mut Chunk> {
        self.current_chunk_origin
            .and_then(move |chunk_origin| self.globe.chunks_mut().get_mut(&chunk_origin))
    }

    fn set_current_chunk(&mut self, new_chunk_origin: ChunkOrigin) {
        self.current_chunk_origin = Some(new_chunk_origin);
        self.current_chunk_might_be_dirty = false;
    }
}
