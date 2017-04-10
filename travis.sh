#!/bin/sh
cd $TRAVIS_BUILD_DIR/planetkit
cargo build --verbose
cargo test --verbose
