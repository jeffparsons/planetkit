#!/bin/bash

# Bold, red text
tput bold
tput setaf 1

echo
echo "This doesn't actually work yet. Not even a little bit. Don't expect it to."
echo

# Non-bold, normal text
tput sgr0

# Run the actual build.
rustup run nightly cargo build --target wasm32-unknown-emscripten "$@"

