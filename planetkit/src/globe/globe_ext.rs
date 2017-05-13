// "Extension" functions for `Globe`. These do not use any private details of `Globe`,
// and so they are exposed on its _inherent impl_ for convenience only.

use rand::Rng;

use grid::{ CellPos, PosInOwningRoot, IntCoord };
use grid::random_column;
use super::chunk::Material;
use super::CursorMut;
use super::globe::Globe;

impl Globe {
    /// Attempt to find dry land at surface level. See `find_dry_land`.
    pub fn find_surface_dry_land(
        &mut self,
        column: CellPos,
        min_air_cells_above: IntCoord,
        max_distance_from_starting_point: IntCoord,
    ) -> Option<CellPos> {
        // Use land height from world gen to approximate cell position where we might find land.
        let land_height = self.gen.land_height(column);
        let approx_cell_z = self.spec().approx_cell_z_from_radius(land_height);
        // Augment column with approximate z-value.
        let mut pos = column;
        pos.z = approx_cell_z;
        // Pass the buck.
        self.find_dry_land(pos, min_air_cells_above, max_distance_from_starting_point)
    }

    /// Attempt to find dry land near (above or below) the given `pos`.
    ///
    /// A land cell position will only be returned if it has at least as many
    /// contiguous cells of air directly above it as specified by `min_air_cells_above`.
    ///
    /// Returns `None` if no such cell can be found within the maximum distance given, e.g.,
    /// if the highest land was below water, or our guess about where there should be land
    /// exposed to air turned out to be wrong.
    ///
    /// Note that this returns the position of the cell found containing land, not the first cell
    /// above it containing air. If you are trying to find a suitable location to, e.g., spawn new
    /// entities, then you probably want to use the position one above the position returned by this function.
    pub fn find_dry_land(
        &mut self,
        start_pos: CellPos,
        min_air_cells_above: IntCoord,
        max_distance_from_starting_point: IntCoord,
    ) -> Option<CellPos> {
        // Interleave searching up and down at the same time. Start the "down" search at the
        // given `start_pos`, and the "up" search one above it.
        let mut distance_from_start: IntCoord = 0;
        let mut down_pos = start_pos;
        let mut up_pos = start_pos;
        up_pos.z += 1;
        // Share a cursor for searching up and down.
        let start_pos_in_owning_root = PosInOwningRoot::new(start_pos, self.spec().root_resolution);
        let chunk_origin = self.origin_of_chunk_owning(start_pos_in_owning_root);
        let mut cursor = CursorMut::new_in_chunk(self, chunk_origin);
        while distance_from_start <= max_distance_from_starting_point {
            'candidate_land: for hopefully_land_pos in &[down_pos, up_pos] {
                if hopefully_land_pos.z < 0 {
                    continue;
                }
                // If it's not land, then we're not interested.
                cursor.set_pos(*hopefully_land_pos);
                cursor.ensure_chunk_present();
                {
                    // Non-lexical lifetimes SVP.
                    let cell = cursor.cell().expect("We just ensured the chunk is present, but apparently it's not. Kaboom!");
                    if cell.material != Material::Dirt {
                        continue;
                    }
                }
                // Ensure minimum required air above.
                let mut hopefully_air_pos = *hopefully_land_pos;
                for _ in 0..min_air_cells_above {
                    hopefully_air_pos.z = hopefully_air_pos.z + 1;
                    cursor.set_pos(hopefully_air_pos);
                    cursor.ensure_chunk_present();
                    let cell = cursor.cell().expect("We just ensured the chunk is present, but apparently it's not. Kaboom!");
                    if cell.material != Material::Air {
                        continue 'candidate_land;
                    }
                }
                // Hurrah! This land cell passed the gauntlet.
                return Some(*hopefully_land_pos);
            }
            down_pos.z -= 1;
            up_pos.z += 1;
            distance_from_start += 1;
        }
        None
    }

    /// Find a random cell immediately above dry land. See `find_dry_land`.
    ///
    /// This is useful for finding a suitable point on the surface of the planet to place new entities,
    /// e.g., the player character when choosing a random spawn point on the planet.
    ///
    /// Returns `None` if no suitable cell could be found within the maximum number of attempts.
    /// Each attempt begins from a new random column.
    pub fn air_above_random_surface_dry_land<R: Rng>(
        &mut self,
        rng: &mut R,
        min_air_cells_above: IntCoord,
        max_distance_from_starting_point: IntCoord,
        max_attempts: usize,
    ) -> Option<CellPos> {
        let mut attempts_remaining = max_attempts;
        while attempts_remaining > 0 {
            let column = random_column(self.spec().root_resolution, rng);
            let maybe_pos = self.find_surface_dry_land(column, min_air_cells_above, max_distance_from_starting_point);
            if let Some(mut pos) = maybe_pos {
                // We want the air above the land we found.
                pos.z += 1;
                return Some(pos);
            }
            attempts_remaining -= 1;
        }
        None
    }

    // TODO: this is not sufficient for finding a suitable place
    // to put a cell dweller; i.e. we need something that randomly
    // samples positions to find a column with land at the top,
    // probably by using the `Gen` to find an approximate location,
    // and then working up and down at the same time to find the
    // closest land to the "surface".
    //
    // TODO: now that `air_above_random_surface_dry_land` exists,
    // track down and destroy all uses of this.
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
