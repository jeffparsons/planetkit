#!/bin/sh
cd $TRAVIS_BUILD_DIR/planetkit
cargo build --release --verbose
cargo test --release --verbose
