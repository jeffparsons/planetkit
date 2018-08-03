#!/bin/bash

set -e

cargo run --release "$@" -- listen &
sleep 1
cargo run --release "$@" -- connect 127.0.0.1:62831
