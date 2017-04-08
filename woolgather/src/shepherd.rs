use specs;
use pk;
use pk::types::*;
use pk::globe;

/// Create the player character: a shepherd who must find and rescue the sheep
/// that have strayed from his flock and fallen into holes.
pub fn create_now(world: &mut specs::World, globe_entity: specs::Entity, globe_spec: pk::globe::Spec) -> specs::Entity {
    use specs::Gate;
    use pk::globe::{ CellPos, Dir };
    use pk::globe::chunk::Material;
    let shepherd_pos = {
        let shepherd_column = CellPos::default();
        let mut globes = world.write::<globe::Globe>().pass();
        let mut globe = globes
            .get_mut(globe_entity)
            .expect("Uh oh, where did our Globe go?");
        globe.find_lowest_cell_containing(shepherd_column, Material::Air)
            .expect("Uh oh, there's something wrong with our globe.")
    };
    world.create_now()
        .with(pk::cell_dweller::CellDweller::new(
            shepherd_pos,
            Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        // The CellDweller's transformation will be set based on its coordinates in cell space.
        .with(pk::Spatial::new(globe_entity, Iso3::identity()))
        .build()
}
