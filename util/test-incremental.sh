#!/bin/bash

# Incremental compilation is getting a lot of attention on nightly,
# so use that day-to-day unless it breaks.
CARGO_INCREMENTAL=1 cargo +nightly test "$@"
