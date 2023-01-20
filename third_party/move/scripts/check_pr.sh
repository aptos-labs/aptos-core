#!/bin/bash
# Copyright (c) The Move Contributors
# SPDX-License-Identifier: Apache-2.0
#
# A script to check whether a local commit is ready for a PR.
# This simulates CI checks locally

set -e

BUILD_FLAGS=

BASE=$(git rev-parse --show-toplevel)
echo "*************** [check-pr] Assuming move root at $BASE"

# Run only tests which would also be run on CI
export ENV_TEST_ON_CI=1

while getopts "htcgdea" opt; do
  case $opt in
    h)
      cat <<EOF
Performs CI equivalent checks on a local client
Usage:
    check_pr <flags>
Flags:
    -h   Print this help
    -t   Run tests
    -c   Run xclippy, xlint, and xfmt
    -g   Run the Move git checks script (whitespace check). This works
         only for committed clients.
    -d   Run documentation generation, abi generation, etc. for move-stdlib
         and other Move libraries.
    -e   Run hardhat EVM tests
    -a   All of the above
    With no options script behaves like -tcg is given.
    If you want to run tests in release mode, call this script as 'BUILD_FLAGS=-release <script>'.


EOF
      exit 1
      ;;
    t)
      TEST=1
      ;;
    c)
      CHECK=1
      ;;
    d)
      GEN_ARTIFACTS=1
      ;;
    g)
      GIT_CHECKS=1
      ;;
    e)
      HARDHAT_CHECKS=1
      ;;
    a)
      TEST=1
      CHECK=1
      GEN_ARTIFACTS=1
      GIT_CHECKS=1
      HARDHAT_CHECKS=1
  esac
done

if [ "$OPTIND" -eq 1 ]; then
  TEST=1
  CHECK=1
  GIT_CHECKS=1
fi

ARTIFACT_CRATES="\
  $BASE/language/move-stdlib\
"

if [ ! -z "$TEST" ]; then
  echo "*************** [check-pr] Running tests"
  (
    cd $BASE
    cargo test --workspace $BUILD_FLAGS
  )
fi

if [ ! -z "$CHECK" ]; then
  echo "*************** [check-pr] Running checks"
  (
    cd $BASE
    cargo xlint
    cargo xclippy --workspace --all-targets
    cargo xfmt
  )
fi

if [ ! -z "$GEN_ARTIFACTS" ]; then
  for dir in $ARTIFACT_CRATES; do
    echo "*************** [check-pr] Generating artifacts for crate $dir"
    (
      cd $dir
      cargo run $BUILD_FLAGS
    )
  done
fi

if [ ! -z "$GIT_CHECKS" ]; then
   echo "*************** [check-pr] Running git checks"
   $BASE/scripts/git-checks.sh
fi

if [ ! -z "$HARDHAT_CHECKS" ]; then
  echo "*************** [check-pr] Running hardhat tests (expecting hardhat configured)"
  (
     cd $BASE/language/tools/move-cli
     cargo install --path . --features evm-backend
  )
  # (
  #   cd $BASE/language/evm/hardhat-move
  #   npm install
  #   npm run build
  # )
  (
    cd $BASE/language/evm/hardhat-examples
    # ./setup.sh
    npx hardhat test
  )
fi
