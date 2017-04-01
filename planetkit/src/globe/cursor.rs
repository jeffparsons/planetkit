use super::{ Globe, CellPos, ChunkOrigin };
use super::chunk::{ Chunk, Cell };

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
    pos: CellPos,
    // Avoid needing to find current chunk again when moving to another
    // pos if we're moving into another pos in the same chunk.
    //
    // May be `None` if the chunk pointed at is not loaded.
    current_chunk: Option<&'a Chunk>,
    current_chunk_might_be_dirty: bool,
}

impl<'a> Cursor<'a> {
    // NOTE: do not make any function here just called `new`;
    // it's super-easy to misuse `Cursor`s, because a lot of the
    // time you really will mean to read cells from a specific
    // chunk, even if that's not the owner of the cell, or the most
    // obvious chunk when you truncate the position.

    /// Only use this if you don't care about which chunk
    /// we'll attempt to read cells from, keeping in mind that
    /// cells at the edge of chunks are shared between chunks.
    ///
    /// If you know that one particular chunk containing a given
    /// cell is loaded, and you want to read from that chunk, you should
    /// use `new_in_chunk` instead.
    fn new_without_chunk_hint(globe: &'a Globe, pos: CellPos) -> Cursor<'a> {
        Cursor {
            globe: globe,
            pos: pos,
            current_chunk: None,
            current_chunk_might_be_dirty: true,
        }
    }

    /// Creates a new cursor at the origin of the given chunk.
    ///
    /// Use this if you know that a particular chunk is loaded,
    /// and you want to read cells from that chunk rather than
    /// any neighboring chunk that might share the same cells.
    pub fn new_in_chunk(globe: &'a Globe, chunk_origin: ChunkOrigin) -> Cursor<'a> {
        let mut cursor = Cursor::new_without_chunk_hint(globe, chunk_origin.into());
        cursor.update_current_chunk();
        cursor
    }

    pub fn pos(&self) -> CellPos {
        self.pos
    }

    pub fn set_pos(&mut self, new_pos: CellPos) {
        self.pos = new_pos;
        self.current_chunk_might_be_dirty = true;
    }

    pub fn globe(&self) -> &'a Globe {
        self.globe
    }

    /// Get a reference to the cell the cursor is currently pointing at.
    ///
    /// Note that this cell might come from a chunk that doesn't own it,
    /// unless you've deliberately ensured you pointed at the cell in its
    /// owning root.
    ///
    /// Returns `None` if the requested cell is in a chunk that isn't loaded.
    pub fn cell(&mut self) -> Option<&'a Cell> {
        // Find the owning chunk if necessary.
        self.update_current_chunk();
        self.current_chunk
            .map(|chunk| chunk.cell(self.pos))
    }

    // Sets `self.current_chunk` to `None` if the cell pointed
    // at is in a chunk that isn't loaded.
    fn update_current_chunk(&mut self) {
        if !self.current_chunk_might_be_dirty {
            // Nothing interesting has happened since we last
            // update the current chunk.
            return;
        }

        if let Some(current_chunk) = self.current_chunk {
            if current_chunk.contains_pos(self.pos) {
                // No need to change chunk; current chunk still contains pos.
                self.current_chunk_might_be_dirty = false;
                return;
            }
        }

        // We either have no current chunk or the current pos is no longer
        // within its bounds.
        //
        // Find a chunk that contains pos. Note that it probably won't be
        // the chunk that _owns_ pos.
        use super::globe::GlobeGuts;
        let chunk_origin = self.globe.origin_of_chunk_in_same_root_containing(self.pos);
        self.current_chunk = self.globe.chunks().get(&chunk_origin);
        self.current_chunk_might_be_dirty = false;
    }
}
