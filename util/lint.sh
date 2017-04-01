#!/bin/bash

# Clippy currently only works on nightly.
# (This might eventually change if it gets folded into the official Rust distribution.)

rustup run nightly cargo clippy "$@"
