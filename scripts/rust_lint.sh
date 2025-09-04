#!/bin/sh

# This assumes you have already installed cargo-sort and cargo machete:
# cargo install cargo-sort
# cargo install cargo-machete
#
# The best way to do this however is to run scripts/dev_setup.sh
#
# If you want to run this from anywhere in velor-core, try adding the wrapper
# script to your path:
# https://gist.github.com/banool/e6a2b85e2fff067d3a215cbfaf808032

# Make sure we're in the root of the repo.
if [ ! -d ".github" ]
then
    echo "Please run this from the root of velor-core!"
    exit 1
fi

# Run in check mode if requested.
CHECK_ARG=""
if [ "$1" = "--check" ]; then
    CHECK_ARG="--check"
fi

# Set appropriate script flags.
set -e
set -x

# Run clippy with the velor-core specific configuration.
cargo xclippy

# Run the formatter. Note: we require the nightly
# build of cargo fmt to provide stricter rust formatting.
cargo +nightly fmt $CHECK_ARG

# Once cargo-sort correctly handles workspace dependencies,
# we can move to cleaner workspace dependency notation.
# See: https://github.com/DevinR528/cargo-sort/issues/47
cargo sort --grouped --workspace $CHECK_ARG

# Check for unused rust dependencies.
cargo machete
