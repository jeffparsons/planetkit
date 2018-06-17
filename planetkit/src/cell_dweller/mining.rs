use super::CellDweller;
use movement::*;
use grid::PosInOwningRoot;
use globe::chunk::{Cell, Material};
use globe::Globe;

/// Assumes that the given CellDweller is indeed attached to the given globe.
/// May panick if this is not true.
pub fn can_pick_up(cd: &mut CellDweller, globe: &mut Globe) -> bool {
    // Only allow picking stuff up if you're sitting above solid ground.
    // (Or, rather, the stuff we consider to be solid for now,
    // which is anything other than air.)
    //
    // TODO: abstract this whole thing... you need some kind of
    // utilities for a globe.
    if cd.pos.z < 0 {
        // There's nothing below; someone built a silly globe.
        return false;
    }
    let under_pos = cd.pos.with_z(cd.pos.z - 1);
    {
        // Inner scope to fight borrowck.
        let under_cell = match globe.maybe_non_authoritative_cell(under_pos) {
            Ok(cell) => cell,
            // Chunk not loaded; wait until it is before attempting to pick up.
            Err(_) => return false,
        };
        if under_cell.material != Material::Dirt {
            return false;
        }
    }

    // Ask the globe if there's anything in front of us to "pick up".
    let mut new_pos = cd.pos;
    let mut new_dir = cd.dir;
    move_forward(&mut new_pos, &mut new_dir, globe.spec().root_resolution)
        .expect("CellDweller should have been in good state.");
    let anything_to_pick_up = {
        // Chunk might not be loaded; in that case assume nothing to pick up.
        globe.maybe_non_authoritative_cell(new_pos).map(|cell| {
            cell.material == Material::Dirt
        }).unwrap_or(false)
    };
    // Also require that there's air above the block;
    // in my initial use case I don't want to allow mining below
    // the surface.
    let air_above_target = {
        // Chunk might not be loaded; in that case assume not air above block.
        let above_new_pos = new_pos.with_z(new_pos.z + 1);
        globe.maybe_non_authoritative_cell(above_new_pos).map(|cell| {
            cell.material == Material::Air
        }).unwrap_or(false)
    };
    anything_to_pick_up && air_above_target
}

// If anything was picked up, then return the position we picked up,
// and what was in it.
pub fn pick_up_if_possible(cd: &mut CellDweller, globe: &mut Globe) -> Option<(PosInOwningRoot, Cell)> {
    if !can_pick_up(cd, globe) {
        return None;
    }

    let mut new_pos = cd.pos;
    let mut new_dir = cd.dir;
    move_forward(&mut new_pos, &mut new_dir, globe.spec().root_resolution)
        .expect("CellDweller should have been in good state.");

    // TODO: make a special kind of thing you can pick up.
    // TODO: accept that as a system argument, and have some builders
    // that make it super-easy to configure.
    // The goal here should be that the "block dude" game
    // ends up both concise and legible.
    let new_pos_in_owning_root =
        PosInOwningRoot::new(new_pos, globe.spec().root_resolution);

    let removed_cell = remove_block(globe, new_pos_in_owning_root);

    // We picked something up.
    Some((new_pos_in_owning_root, removed_cell))
}

pub fn remove_block(globe: &mut Globe, pos_in_owning_root: PosInOwningRoot) -> Cell {
    use globe::is_point_shared;

    // Keep for later, so we can return what was in it.
    let cloned_cell = {
        let cell = globe.authoritative_cell_mut(pos_in_owning_root);
        let cs = cell.clone();
        cell.material = Material::Air;
        cs
    };

    // Some extra stuff is only relevant if the cell is shared
    // with another chunk (horizontal edges).
    if is_point_shared(
        *pos_in_owning_root.pos(),
        globe.spec().chunk_resolution,
    )
    {
        // Bump version of owned shared cells.
        globe.increment_chunk_owned_edge_version_for_cell(pos_in_owning_root);
        // Propagate change to neighbouring chunks.
        let chunk_origin = globe.origin_of_chunk_owning(pos_in_owning_root);
        globe.push_shared_cells_for_chunk(chunk_origin);
    }
    // Mark the view for the containing chunk and those containing each cell surrounding
    // it as being dirty. (This cell might affect the visibility of cells in those chunks.)
    // TODO: different API where you commit to changing a cell
    // in a closure you get back that has a reference to it?
    // Or contains a _wrapper_ around it so it knows if you mutated it? Ooooh.
    globe.mark_chunk_views_affected_by_cell_as_dirty(pos_in_owning_root.into());
    // TODO: remember on the cell-dweller that it's carrying something?
    // Or should that be a different kind of component?

    cloned_cell
}
