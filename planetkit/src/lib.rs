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

extern crate bytes;
extern crate chrono;
extern crate glutin_window;
extern crate graphics;
extern crate noise;
extern crate opengl_graphics;
extern crate piston;
extern crate rand;
#[macro_use]
extern crate gfx;
extern crate camera_controllers;
extern crate gfx_device_gl;
extern crate nalgebra as na;
extern crate ncollide3d;
extern crate nphysics3d;
extern crate piston_window;
extern crate shader_version;
extern crate vecmath;
#[macro_use]
extern crate slog;
extern crate shred;
extern crate slog_async;
extern crate slog_term;
#[macro_use]
extern crate shred_derive;
extern crate num_traits;
extern crate specs;
#[macro_use]
extern crate itertools;
#[cfg(test)]
#[macro_use]
extern crate approx;
extern crate arrayvec;
extern crate froggy;
extern crate futures;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

// Stuff we can't run on the web yet.
#[cfg(not(target_os = "emscripten"))]
extern crate tokio_codec;
#[cfg(not(target_os = "emscripten"))]
extern crate tokio_core;
#[cfg(not(target_os = "emscripten"))]
extern crate tokio_io;

#[cfg(all(feature = "nightly", test))]
extern crate test;

pub mod app;
pub mod camera;
pub mod cell_dweller;
pub mod globe;
pub mod grid;
pub mod input_adapter;
pub mod movement;
pub mod net;
pub mod physics;
pub mod render;
pub mod simple;
pub mod types;
pub mod window;

mod spatial;
pub use spatial::Spatial;
pub use spatial::SpatialStorage;

mod log_resource;
pub use log_resource::LogResource;

mod app_builder;
pub use app_builder::AppBuilder;

#[cfg(test)]
mod integration_tests;
