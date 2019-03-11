#!/bin/bash

set -ex

cargo test --release
cargo fmt
cargo clippy --bins --examples --tests --benches -- -D warnings
