#!/bin/sh

# This assumes you have already installed cargo-sort:
# cargo install cargo-sort
#
# The best way to do this however is to run scripts/dev_setup.sh
#
# If you want to run this from anywhere in aptos-core, try adding this wrapepr
# script to your path:
# https://gist.github.com/banool/e6a2b85e2fff067d3a215cbfaf808032

# Make sure we're in the root of the repo.
if [ ! -d ".github" ] 
then
    echo "Please run this from the root of aptos-core" 
    exit 1
fi

# Run in check mode if requested.
CHECK_ARG=""
if [ "$1" = "--check" ]; then
    CHECK_ARG="--check"
fi

set -e
set -x

cargo xclippy
cargo fmt $CHECK_ARG
cargo sort --grouped --workspace $CHECK_ARG
