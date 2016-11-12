use noise;

use super::spec::Spec;
use super::chunk::{ CellPos, Cell, Material };

// Globe content generator. Stores all the state for generating
// the terrain and any other parts of the globe that are derived
// from its seed.
//
// Will eventually do some basic caching, etc., but is pretty dumb
// right now.
pub struct Gen {
    spec: Spec,
    // Permutation table for noise
    pt: noise::Seed,
}

impl Gen {
    pub fn new(spec: Spec) -> Gen {
        assert!(spec.is_valid(), "Invalid globe spec!");
        let pt = noise::Seed::new(spec.seed);
        Gen {
            spec: spec,
            pt: pt,
        }
    }

    pub fn cell_at(&self, cell_pos: CellPos) -> Cell {
        // TODO: get parameters from spec
        //
        // TODO: store this function... when you figure
        // out what's going on with the types.
        // ("expected fn pointer, found fn item")
        let terrain_noise = noise::Brownian3::new(
            noise::open_simplex3::<f64>, 6
        ).wavelength(1.0);

        // Calculate height for this cell from world spec.
        // To do this, project the cell onto a unit sphere
        // and sample 3D simplex noise to get a height value.
        //
        // TODO: split out a proper world generator
        // that layers in lots of different kinds of noise etc.
        let land_pt3 = self.spec.cell_center_on_unit_sphere(cell_pos);
        let cell_pt3 = self.spec.cell_center_center(cell_pos);

        // Vary a little bit around 1.0.
        let delta =
            terrain_noise.apply(&self.pt, land_pt3.as_ref())
            * self.spec.ocean_radius
            * 0.3;
        let land_height = self.spec.ocean_radius + delta;
        // TEMP: ...
        use na::Norm;
        let cell_height = cell_pt3.as_vector().norm();
        let material = if cell_height < land_height {
            Material::Dirt
        } else if cell_height < self.spec.ocean_radius {
            Material::Water
        } else {
            Material::Air
        };
        Cell {
            material: material,
            // `Globe` fills this in; it's not really a property
            // of the naturally generated world, and it's not
            // deterministic from the world seed, so we don't
            // want to pollute `Gen` with it.
            //
            // TODO: probably remove this? We're just using
            // temporarily to create some texture across
            // cells to make them easy to tell apart and look
            // kinda nice, but this probably isn't a great
            // long-term solution...
            shade: 1.0,
        }
    }
}
