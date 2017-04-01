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
cargo +nightly rustc --target wasm32-unknown-emscripten --bin demo -- -C link-arg="-s" -C link-arg="BINARYEN_METHOD='interpret-binary'"

