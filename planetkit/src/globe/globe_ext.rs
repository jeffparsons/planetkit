// "Extension" functions for `Globe`. These do not use any private details of `Globe`,
// and so they are exposed on its _inherent impl_ for convenience only.

use super::{ CellPos, PosInOwningRoot };
use super::chunk::Material;
use super::CursorMut;
use super::globe::Globe;

impl Globe {
    // TODO: this is not sufficient for finding a suitable place
    // to put a cell dweller; i.e. we need something that randomly
    // samples positions to find a column with land at the top,
    // probably by using the `Gen` to find an approximate location,
    // and then working up and down at the same time to find the
    // closest land to the "surface".
    pub fn find_lowest_cell_containing(
        &mut self,
        column: CellPos,
        material: Material
    ) -> CellPos {
        // Translate into owning root, then start at bedrock.
        let mut column = PosInOwningRoot::new(column, self.spec().root_resolution);
        column.set_z(0);
        let chunk_origin = self.origin_of_chunk_owning(column);
        let mut cursor = CursorMut::new_in_chunk(self, chunk_origin);
        cursor.set_pos(column.into());

        loop {
            // TODO: cursor doesn't guarantee you're reading authoritative data.
            // Do we care about that? Do we just need to make sure that "ensure chunk"
            // loads any other chunks that might be needed? But gah, then you're going to
            // have a chain reaction, and load ALL chunks. Maybe it's Cursor's
            // responsibility, then. TODO: think about this. :)
            //
            // Maybe you need a special kind of cursor. That only looks at owned cells
            // and automatically updates itself whenever you set its position.
            //
            // TODO: "collect garbage" occasionally? Or every iteration, even.
            cursor.ensure_chunk_present();
            {
                let pos = cursor.pos();
                let cell = cursor.cell().expect("We just ensured the chunk is present, but apparently it's not. Kaboom!");
                if cell.material == material {
                    // Yay, we found it!
                    return pos.into();
                }
            }
            let new_pos = cursor.pos().set_z(cursor.pos().z + 1);
            cursor.set_pos(new_pos);
        }
    }
}
