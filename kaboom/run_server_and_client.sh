#!/bin/bash -e

cargo run "$@" -- listen &
sleep 1
cargo run "$@" -- connect 127.0.0.1:62831
