use specs::{self, Entity, ReadStorage, WriteStorage};

use super::types::*;

/// Translation and rotation relative to some parent entity.
///
/// The only exception is a "root" entity, which has no parent,
/// and acts as the top of a tree of all entities whose transformation
/// can be expressed relative to each other. (Any entities not sharing
/// the same root have no meaningful spatial relationship to each other.)
pub struct Spatial {
    local_transform: Iso3,
    parent_entity: Option<Entity>,
}

impl Spatial {
    pub fn new(parent_entity: Entity, local_transform: Iso3) -> Spatial {
        Spatial {
            parent_entity: Some(parent_entity),
            local_transform: local_transform,
        }
    }

    pub fn new_root() -> Spatial {
        use num_traits::One;
        Spatial {
            parent_entity: None,
            local_transform: Iso3::one(),
        }
    }

    pub fn local_transform(&self) -> Iso3 {
        self.local_transform
    }

    pub fn set_local_transform(&mut self, new_local_transform: Iso3) {
        self.local_transform = new_local_transform;
    }

    pub fn parent_entity(&self) -> Option<Entity> {
        self.parent_entity
    }
}

impl specs::Component for Spatial {
    type Storage = specs::VecStorage<Spatial>;
}

pub trait SpatialStorage {
    // Signed so we can do tricksy math without casting.
    fn depth_of(&self, entity: Entity) -> i32;
    fn root_of(&self, entity: Entity) -> Entity;
    fn have_common_ancestor(&self, a: Entity, b: Entity) -> bool;
    fn lowest_common_ancestor(&self, a: Entity, b: Entity) -> Entity;
    fn a_relative_to_b(&self, a: Entity, b: Entity) -> Iso3;
    fn a_relative_to_ancestor_b(&self, a: Entity, b: Entity) -> Iso3;
    fn a_local_transform_relative_to_ancestor_b(
        &self,
        a: Entity,
        a_local_transform: Iso3,
        b: Entity,
    ) -> Iso3;
}

// This is a gross hack to abstract over mutability of storages,
// because `specs::MaskedStorage` was made private in Specs 0.9.
//
// TODO: It's hard to argue that `specs::MaskedStorage` should remain in the
// public interface just for my needs, but maybe there's a case for a trait
// like this one existing in Specs. Create an issue to discuss...
pub trait MaybeMutStorage<'a, T> {
    fn get(&self, e: Entity) -> Option<&T>;
}

impl<'a, T> MaybeMutStorage<'a, T> for ReadStorage<'a, T>
where
    T: specs::Component,
{
    fn get(&self, e: Entity) -> Option<&T> {
        (self as &ReadStorage<T>).get(e)
    }
}

impl<'a, T> MaybeMutStorage<'a, T> for WriteStorage<'a, T>
where
    T: specs::Component,
{
    fn get(&self, e: Entity) -> Option<&T> {
        (self as &WriteStorage<T>).get(e)
    }
}

impl<'e, S> SpatialStorage for S
where
    S: MaybeMutStorage<'e, Spatial>,
{
    fn depth_of(&self, entity: Entity) -> i32 {
        let spatial = self
            .get(entity)
            .expect("Given entity doesn't have a Spatial");
        match spatial.parent_entity {
            None => 0,
            Some(parent) => 1 + self.depth_of(parent),
        }
    }

    fn root_of(&self, entity: Entity) -> Entity {
        let spatial = self
            .get(entity)
            .expect("Given entity doesn't have a Spatial");
        match spatial.parent_entity {
            None => entity,
            Some(parent) => self.root_of(parent),
        }
    }

    fn have_common_ancestor(&self, a: Entity, b: Entity) -> bool {
        self.root_of(a) == self.root_of(b)
    }

    // TODO: Add a bunch of `debug_assert`s throughout to make sure
    // that the arguments you're getting are legit for these functions!
    //
    // TODO: consider returning Option<Entity> in case they have no common ancestor.
    fn lowest_common_ancestor(&self, mut a: Entity, mut b: Entity) -> Entity {
        debug_assert!(self.have_common_ancestor(a, b));

        // If one `Spatial` is deeper than the other, then the path to the lowest common
        // ancestor from the deeper will necessarily pass through a `Spatial` at the same
        // depth as the shallower node.
        //
        // Note that this will also _be_ the shallower `Spatial` if the shallower
        // is an ancestor of the deeper, or if both are actually the same.
        let mut depth_delta = self.depth_of(a) - self.depth_of(b);
        while depth_delta > 0 {
            // If `a` was deeper, find its ancestor with same height as `b`.
            a = self
                .get(a)
                .expect("Entity isn't a Spatial")
                .parent_entity
                .expect("I thought this Spatial had a parent...");
            depth_delta -= 1;
        }
        while depth_delta < 0 {
            // If `b` was deeper, find its ancestor with same height as `a`.
            b = self
                .get(b)
                .expect("Entity isn't a Spatial")
                .parent_entity
                .expect("I thought this Spatial had a parent...");
            depth_delta += 1;
        }

        // Now that the two nodes whose parent ancestor we seek lie at the same
        // depth, it's just a matter of ascending each path one step at a time
        // until they intersect.
        while a != b {
            a = self
                .get(a)
                .expect("Entity isn't a Spatial")
                .parent_entity
                .expect(
                    "I thought this Spatial had a parent; maybe a and b do not share a root...",
                );
            b = self
                .get(b)
                .expect("Entity isn't a Spatial")
                .parent_entity
                .expect(
                    "I thought this Spatial had a parent; maybe a and b do not share a root...",
                );
        }
        a
    }

    fn a_relative_to_b(&self, a: Entity, b: Entity) -> Iso3 {
        let lca = self.lowest_common_ancestor(a, b);
        let a_relative_to_lca = self.a_relative_to_ancestor_b(a, lca);
        let b_relative_to_lca = self.a_relative_to_ancestor_b(b, lca);
        b_relative_to_lca.inverse() * a_relative_to_lca
    }

    fn a_relative_to_ancestor_b(&self, a: Entity, b: Entity) -> Iso3 {
        self.a_local_transform_relative_to_ancestor_b(a, Iso3::identity(), b)
    }

    fn a_local_transform_relative_to_ancestor_b(
        &self,
        a: Entity,
        a_local_transform: Iso3,
        b: Entity,
    ) -> Iso3 {
        if a == b {
            a_local_transform
        } else {
            // TODO: factor this out; it seems to be a common pattern...
            let a_spatial = self.get(a).expect("Entity isn't a Spatial");
            let parent = a_spatial
                .parent_entity
                .expect("I thought this Spatial had a parent...");
            self.a_local_transform_relative_to_ancestor_b(parent, a_spatial.local_transform(), b)
                * a_local_transform
        }
    }
}

#[cfg(test)]
mod tests {
    use na;
    use specs::{self, Builder};

    use super::*;

    // Test data to reuse across tests, so we can start
    // asserting really simple things and work up from there.
    //
    // Convention here is that XY is the orbital plane.
    //
    // I'm not trying to make units make sense.
    struct SolarSystem {
        pub world: specs::World,
        pub sun: specs::Entity,
        pub earth: specs::Entity,
        pub moon: specs::Entity,
        // Polar satellite looking down.
        pub polar_satellite: specs::Entity,
    }

    impl SolarSystem {
        pub fn new() -> SolarSystem {
            let mut world = specs::World::new();
            world.register::<Spatial>();

            let sun = world.create_entity().with(Spatial::new_root()).build();

            let earth_transform = Iso3::new(Vec3::new(1000.0, 2000.0, 0.0), na::zero());
            let earth = world
                .create_entity()
                .with(Spatial::new(sun, earth_transform))
                .build();

            let moon_transform = Iso3::new(Vec3::new(300.0, 400.0, 0.0), na::zero());
            let moon = world
                .create_entity()
                .with(Spatial::new(earth, moon_transform))
                .build();

            let eye = Pt3::new(0.0, 0.0, 10.0);
            let target = Pt3::origin();
            // Look straight down at Earth.
            let polar_satellite_transform = Iso3::look_at_rh(&eye, &target, &Vec3::y());
            let polar_satellite = world
                .create_entity()
                .with(Spatial::new(earth, polar_satellite_transform))
                .build();

            SolarSystem {
                world: world,
                sun: sun,
                earth: earth,
                moon: moon,
                polar_satellite: polar_satellite,
            }
        }
    }

    #[test]
    fn pos_relative_to_parent() {
        let ss = SolarSystem::new();
        let spatials = ss.world.read_storage::<Spatial>();

        let earth_from_sun = spatials.a_relative_to_b(ss.earth, ss.sun);
        assert_relative_eq!(
            earth_from_sun.translation.vector,
            Vec3::new(1000.0, 2000.0, 0.0),
        );

        let sun_from_earth = spatials.a_relative_to_b(ss.sun, ss.earth);
        assert_relative_eq!(
            sun_from_earth.translation.vector,
            Vec3::new(-1000.0, -2000.0, 0.0),
        );
    }

    #[test]
    fn pos_relative_to_grandparent() {
        let ss = SolarSystem::new();
        let spatials = ss.world.read_storage::<Spatial>();

        let moon_from_sun = spatials.a_relative_to_b(ss.moon, ss.sun);
        assert_relative_eq!(
            moon_from_sun.translation.vector,
            Vec3::new(1300.0, 2400.0, 0.0),
        );

        let sun_from_moon = spatials.a_relative_to_b(ss.sun, ss.moon);
        assert_relative_eq!(
            sun_from_moon.translation.vector,
            Vec3::new(-1300.0, -2400.0, 0.0),
        );
    }

    #[test]
    fn pos_accounting_for_orientation_relative_to_parent() {
        let ss = SolarSystem::new();
        let mut spatials = ss.world.write_storage::<Spatial>();

        let earth_from_polar_satellite = spatials.a_relative_to_b(ss.earth, ss.polar_satellite);
        assert_relative_eq!(
            earth_from_polar_satellite.translation.vector,
            Vec3::new(0.0, 0.0, 10.0),
        );

        // Rotate the Earth a bit and make sure it doesn't make a difference.
        let eye = Pt3::from_coordinates(
            spatials
                .get(ss.earth)
                .unwrap()
                .local_transform()
                .translation
                .vector,
        );
        // Look off to wherever.
        let target = Pt3::new(123.0, 321.0, 456.0);
        let new_earth_transform = Iso3::look_at_rh(&eye, &target, &Vec3::y());
        spatials
            .get_mut(ss.earth)
            .unwrap()
            .set_local_transform(new_earth_transform);

        let earth_from_polar_satellite = spatials.a_relative_to_b(ss.earth, ss.polar_satellite);
        assert_relative_eq!(
            earth_from_polar_satellite.translation.vector,
            Vec3::new(0.0, 0.0, 10.0),
        );
    }
}
