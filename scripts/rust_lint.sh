#!/bin/sh

# This assumes you have already installed cargo-sort:
# cargo install cargo-sort
#
# The best way to do this however is to run scripts/dev_setup.sh

# Make sure we're in the root of the repo.
cd "$(dirname "$0")"
cd ..

# Run in check mode if requested.
CHECK_ARG=""
if [ "$1" = "--check" ]; then
    CHECK_ARG="--check"
fi

set -x

cargo xclippy
cargo fmt $CHECK_ARG
cargo sort --grouped --workspace $CHECK_ARG
