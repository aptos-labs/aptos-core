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

# We require the nightly build of cargo fmt
# to provide stricter rust formatting.
cargo +nightly fmt $CHECK_ARG

# Once cargo-sort correctly handles workspace dependencies,
# we can move to cleaner workspace dependency notation.
# See: https://github.com/DevinR528/cargo-sort/issues/47
cargo sort --grouped --workspace $CHECK_ARG

# Ensure that aptos-cached-packages have been built correctly.
unset SKIP_FRAMEWORK_BUILD
cargo build -p aptos-cached-packages
if [ -n "$CHECK_ARG" ]; then
    if [ -n "$(git status --porcelain -uno aptos-move)" ]; then
      git diff
      echo "There are unstaged changes after running 'cargo build -p aptos-cached-packages'! Are you sure aptos-cached-packages is up-to-date?"
      exit 1
    fi
fi
