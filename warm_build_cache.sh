#!/bin/bash -e

# Build everything in debug and release modes, including all tests
# and benchmarks, to make sure we have all dependencies downloaded
# and everything compiled.
#
# This is mostly useful if, e.g., you've just pulled down a new version
# of the Rust compiler, and don't want surprise long compile wait times later.
# (Especially useful if you're about to hit the road soon and don't
# want to burn through your battery life and/or mobile data quota.)

cargo build --bins --tests --benches --examples
cargo build --bins --tests --benches --examples --release
