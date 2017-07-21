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
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#![cfg_attr(all(feature = "nightly", test), feature(test))]

extern crate chrono;
extern crate rand;
extern crate noise;
extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate piston_window;
extern crate camera_controllers;
extern crate vecmath;
extern crate shader_version;
extern crate nalgebra as na;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;
extern crate shred;
extern crate specs;
extern crate num_traits;
#[macro_use]
extern crate itertools;
#[cfg(test)]
#[macro_use]
extern crate approx;
extern crate froggy;
extern crate arrayvec;
extern crate futures;
#[macro_use]
extern crate tokio_core;

#[cfg(all(feature = "nightly", test))]
extern crate test;

pub mod input_adapter;
pub mod grid;
pub mod globe;
pub mod types;
pub mod app;
pub mod window;
pub mod render;
pub mod simple;
pub mod cell_dweller;
pub mod movement;
pub mod camera;
pub mod network;

mod spatial;
pub use spatial::Spatial;

#[cfg(test)]
mod integration_tests;
