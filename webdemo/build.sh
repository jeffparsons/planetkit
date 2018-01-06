#!/bin/bash -e

# Bold, red text
tput bold
tput setaf 1

echo
echo "This doesn't actually work yet. Don't expect it to."
echo

# Non-bold, normal text
tput sgr0

# Parse command line arguments.
# `getopt` is fiddly, and overly complex for our hacky needs here.
# Just pump args ignoring anything we don't recognise.
while [[ ! -z $1 ]]; do
    if [[ $1 == '--release' ]]; then
        maybe_release='--release'
        echo "Building in release mode."
        shift
    fi

    if [[ $1 == '--nightly' ]]; then
        maybe_nightly='+nightly'
        echo "Building using nightly toolchain."
        shift
    fi
done

if [[ ! -z "${maybe_release}" ]]; then
    dest=target/wasm32-unknown-emscripten/release/
else
    dest=target/wasm32-unknown-emscripten/debug/
fi

cargo $maybe_nightly build $maybe_release --target wasm32-unknown-emscripten

cp index.html "$dest"
pushd "$dest"
python -m SimpleHTTPServer 8123
