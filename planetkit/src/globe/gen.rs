use noise;

use super::spec::Spec;
use super::CellPos;
use super::chunk::{ Cell, Material };

// TODO: turn this into a component that we can slap onto a Globe
// or other globe-oid (distant point?).

/// Globe content generator. Stores all the state for generating
/// the terrain and any other parts of the globe that are derived
/// from its seed.
///
/// Will eventually do some basic caching, etc., but is pretty dumb
/// right now.
///
/// The plan is for this to eventually be used with multiple
/// implementations of globes, e.g., a full voxmap based globe,
/// a distant blob in the sky, to a shiny dot in the distance.
pub struct Gen {
    spec: Spec,
    terrain_noise: noise::Fbm<f64>,
}

impl Gen {
    pub fn new(spec: Spec) -> Gen {
        use noise::Seedable;
        use noise::MultiFractal;

        assert!(spec.is_valid(), "Invalid globe spec!");

        // TODO: get parameters from spec
        //
        // TODO: store this function... when you figure
        // out what's going on with the types.
        // ("expected fn pointer, found fn item")
        //
        // TODO: even more pressing now that the noise API has
        // changed to deprecate PermutationTable; is it now stored
        // within Fbm? This might be super-slow now...
        let terrain_noise = noise::Fbm::<f64>::new()
        // TODO: make wavelength etc. part of spec;
        // the octaves and wavelength of noise you want
        // will probably depend on planet size.
            .set_octaves(6)
            .set_frequency(1.0 / 700.0)
            // TODO: probably allow a bigger seed; what's the smallest usize on any real platform?
            .set_seed(spec.seed as usize);
        Gen {
            spec: spec,
            terrain_noise: terrain_noise,
        }
    }

    pub fn cell_at(&self, cell_pos: CellPos) -> Cell {
        use noise::NoiseModule;

        // Calculate height for this cell from world spec.
        // To do this, project the cell onto a sea-level sphere
        // and sample 3D simplex noise to get a height value.
        //
        // Basing on sea-level lets us use similar wavelengths
        // to similar effect, regardless of the globe radius.
        //
        // TODO: split out a proper world generator
        // that layers in lots of different kinds of noise etc.
        let sea_level_pt3 = self.spec.cell_center_on_unit_sphere(cell_pos)
            * self.spec.ocean_radius;
        let cell_pt3 = self.spec.cell_center_center(cell_pos);

        // Vary a little bit around 1.0.
        let delta = self.terrain_noise.get([sea_level_pt3.x, sea_level_pt3.y, sea_level_pt3.z])
            * (self.spec.ocean_radius - self.spec.floor_radius)
            // TODO: this 0.9 is only to stop the dirt level
            // going below bedrock. Need something a bit more sophisticated
            // than this eventually.
            * 0.9;
        let land_height = self.spec.ocean_radius + delta;
        // TEMP: ...
        let cell_height = cell_pt3.coords.norm();
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
