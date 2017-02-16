use super::{ Globe, CellPos };
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
    pub fn new(globe: &'a Globe, pos: CellPos) -> Cursor<'a> {
        Cursor {
            globe: globe,
            pos: pos,
            current_chunk: None,
            current_chunk_might_be_dirty: true,
        }
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
