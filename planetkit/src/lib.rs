/*!
# PlanetKit

**PlanetKit** is game programming library with a strong focus on:

  - Mutable voxel-based planets
  - Arbitrarily large universes
  - Modularity and composability.

It is intended as a high-level "batteries-included" toolkit for a relatively narrow set of game styles.


# Project status

The project is very young, and in a state of rapid flux.

The API is far from stable, and documentation is sparse at best. In lieu of API stability,
if you do use PlanetKit for anything, I'll do my best to help you deal with the inevitable breakage.

I intend to publish the library to [crates.io](https://crates.io/) as soon as I have a token example game
that uses PlanetKit as any other application would. At the moment, my example application and the
library are too tangled for me to honestly call it a library ready for any kind of third party use.


## High-level design

PlanetKit's architecture is based on the [entity-component system](https://en.wikipedia.org/wiki/Entity%E2%80%93component%E2%80%93system)
pattern, and uses the [Specs](https://slide-rs.github.io/specs/specs/index.html) crate to implement this. Therefore the
primary means of extending PlanetKit and composing different components written for it is through the use of Specs
[`Component`s](https://slide-rs.github.io/specs/specs/trait.Component.html) and [`System`s](https://slide-rs.github.io/specs/specs/trait.System.html).

I am keeping a close eye on [Froggy](https://github.com/kvark/froggy) as a potential replacement for Specs further down
the line. This would imply significant API breakage.
*/

// Hook up Clippy plugin if explicitly requested.
// You should only do this on nightly Rust.
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(all(feature = "nightly", test), feature(test))]

#[macro_use]
extern crate gfx;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate shred_derive;
#[macro_use]
extern crate itertools;
#[macro_use]
extern crate serde_derive;

#[cfg(all(feature = "nightly", test))]
extern crate test;
#[cfg(test)]
#[macro_use]
extern crate approx;

use nalgebra as na;

pub use planetkit_grid as grid;
pub use grid::movement as movement;

pub mod app;
pub mod camera;
pub mod cell_dweller;
pub mod globe;
pub mod input_adapter;
pub mod net;
pub mod physics;
pub mod render;
pub mod simple;
pub mod types;
pub mod window;

mod spatial;
pub use crate::spatial::Spatial;
pub use crate::spatial::SpatialStorage;

mod log_resource;
pub use crate::log_resource::LogResource;

mod app_builder;
pub use crate::app_builder::AppBuilder;

#[cfg(test)]
mod integration_tests;
