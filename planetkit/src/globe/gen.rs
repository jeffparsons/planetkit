use noise;

use super::chunk::{Cell, Material};
use super::spec::Spec;
use crate::globe::ChunkOrigin;
use crate::grid::{GridPoint2, GridPoint3};

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
pub trait Gen: Send + Sync {
    fn land_height(&self, column: GridPoint2) -> f64;
    fn cell_at(&self, grid_point: GridPoint3) -> Cell;
    fn populate_cells(&self, origin: ChunkOrigin, cells: &mut Vec<Cell>);
}

pub struct SimpleGen {
    spec: Spec,
    terrain_noise: noise::Fbm,
}

impl SimpleGen {
    pub fn new(spec: Spec) -> SimpleGen {
        use noise::MultiFractal;
        use noise::Seedable;

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
        let terrain_noise = noise::Fbm::new()
            // TODO: make wavelength etc. part of spec;
            // the octaves and wavelength of noise you want
            // will probably depend on planet size.
            .set_octaves(6)
            .set_frequency(1.0 / 700.0)
            // Truncate seed to make it fit what `noise` expects.
            .set_seed(spec.seed as u32);
        SimpleGen {
            spec: spec,
            terrain_noise: terrain_noise,
        }
    }
}

impl Gen for SimpleGen {
    fn land_height(&self, column: GridPoint2) -> f64 {
        use noise::NoiseFn;

        // Calculate height for this cell from world spec.
        // To do this, project the cell onto a sea-level sphere
        // and sample 3D simplex noise to get a height value.
        //
        // Basing on sea-level lets us use similar wavelengths
        // to similar effect, regardless of the globe radius.
        //
        // TODO: split out a proper world generator
        // that layers in lots of different kinds of noise etc.
        let sea_level_pt3 = self.spec.cell_center_on_unit_sphere(column) * self.spec.ocean_radius;
        // Vary a little bit around 1.0.
        let delta = self.terrain_noise.get([sea_level_pt3.x, sea_level_pt3.y, sea_level_pt3.z])
            * (self.spec.ocean_radius - self.spec.floor_radius)
            // TODO: this 0.9 is only to stop the dirt level
            // going below bedrock. Need something a bit more sophisticated
            // than this eventually.
            //
            // Also... OpenSimplex, which FBM uses, appears to be totally bonkers?
            // https://github.com/brendanzab/noise-rs/issues/149
            * 0.45;
        self.spec.ocean_radius + delta
    }

    fn cell_at(&self, grid_point: GridPoint3) -> Cell {
        let land_height = self.land_height(grid_point.rxy);
        let cell_pt3 = self.spec.cell_center_center(grid_point);
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

    fn populate_cells(&self, origin: ChunkOrigin, cells: &mut Vec<Cell>) {
        use rand;
        use rand::Rng;

        // We should be passed an empty vector to populate.
        assert!(cells.is_empty());

        let chunk_res = &self.spec.chunk_resolution;
        let origin = origin.pos();

        // Include cells _on_ the far edge of the chunk;
        // even though we don't own them we'll need to draw part of them.
        let end_x = origin.x + chunk_res[0];
        let end_y = origin.y + chunk_res[1];
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_z = origin.z + chunk_res[2] - 1;
        for cell_z in origin.z..=end_z {
            for cell_y in origin.y..=end_y {
                for cell_x in origin.x..=end_x {
                    let grid_point = GridPoint3::new(origin.root, cell_x, cell_y, cell_z);
                    let mut cell = self.cell_at(grid_point);
                    // Temp hax?
                    let mut rng = rand::thread_rng();
                    cell.shade = 1.0 - 0.5 * rng.gen::<f32>();
                    cells.push(cell);
                }
            }
        }
    }
}
