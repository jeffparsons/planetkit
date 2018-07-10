#!/bin/bash

set -e

cargo run "$@" -- listen
