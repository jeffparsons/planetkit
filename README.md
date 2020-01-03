# PlanetKit

[![Travis CI build status][bi]][bl]
[![AppVeyor build status][ai]][al]
[![Rust](https://img.shields.io/badge/rust-1.40%2B-blue.svg?maxAge=3600)](https://github.com/jeffparsons/rangemap) <!-- Don't forget to update the Travis config when bumping minimum Rust version. -->

[bi]: https://travis-ci.org/jeffparsons/planetkit.svg?branch=master
[bl]: https://travis-ci.org/jeffparsons/planetkit

[ai]: https://ci.appveyor.com/api/projects/status/vfk0w163ojw8nmdv/branch/master?svg=true
[al]: https://ci.appveyor.com/project/jeffparsons/planetkit/branch/master


Colorful blobs that might one day resemble planets.

Requires Rust 1.40.

![Screenshot](https://raw.githubusercontent.com/jeffparsons/planetkit/master/screenshot.png)


## Goals

- **Build an easily-hackable toolkit** for building games based around voxel globes. The simple case should be simple, and the migration path to greater customisation should be smooth.

- **Document everything**. Both the API and implementation should aim to teach. I'm also [blogging as I go](https://jeffparsons.github.io/).

- **Be open and welcoming**. If you have a question, a suggestion, an idea, or just want to say "hi", please feel free to [open an issue](https://github.com/jeffparsons/planetkit/issues/new?title=Hi%20there!).


## Non-goals

- **Build a game engine**. I'm going to end up doing this accidentally. But my preference is to use existing libraries where they exist, and contribute to them where it makes sense.

- **Expose a stable API**. I do value stability, but PlanetKit is too young and exploratory to be thinking about that just yet. If you use it for something, I'll help you deal with breakage.

- **Meet everyone's needs**. I intend to focus on a very narrow set of game styles. But "narrow" is hard to define clearly. If you're not sure whether our visions are aligned, please open an issue!


## License

PlanetKit is primarily distributed under the terms of both the MIT license
and the Apache License (Version 2.0).

See LICENSE-APACHE and LICENSE-MIT for details.
