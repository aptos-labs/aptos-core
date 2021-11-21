#!/bin/bash
# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

# A script to check whether a local commit related to the Move Prover and Diem Framework
# is ready for a PR.

set -e

BUILD_MODE=--release

BASE=$(git rev-parse --show-toplevel)
echo "*************** [check-pr] Assuming diem root at $BASE"

while getopts "hcxtdgmea" opt; do
  case $opt in
    h)
      cat <<EOF
Usage:
    check_pr <flags>
Flags:
    -h   Print this help
    -c   Check the core prover crates using cargo xfmt/xclippy.
         This is the default if no flags are provided.
    -x   Like -c, but adds more crates (specifically all which depend
         on move-model)
    -t   In addition to xfmt/xclippy, run cargo test
    -d   Run documentation generation, abi generation, etc. for move-stdlib
         and diem-framework.
    -g   Run the Diem git checks script (whitespace check). This works
         only for committed clients.
    -m   Run the Move unit and verification tests.
    -e   Run the Move e2e tests
    -a   Run all of the above
EOF
      exit 1
      ;;
    c)
      CHECK=1
      ;;
    x)
      CHECK=1
      CHECK_MORE=1
      ;;
    d)
      GEN_ARTIFACTS=1
      ;;
    g)
      GIT_CHECKS=1
      ;;
    t)
      ALSO_TEST=1
      ;;
    m)
      MOVE_TESTS=1
      ;;
    e)
      MOVE_E2E_TESTS=1
      ;;
    a)
      CHECK=1
      CHECK_MORE=1
      GEN_ARTIFACTS=1
      GIT_CHECKS=1
      ALSO_TEST=1
      MOVE_TESTS=1
      MOVE_E2E_TESTS=1
      ;;
  esac
done

if [ "$OPTIND" -eq 1 ]; then
  CHECK=1
fi


CRATES="\
  $BASE/language/move-prover/bytecode \
  $BASE/language/move-prover/boogie-backend \
  $BASE/language/move-prover\
  $BASE/language/move-model\
"

if [ ! -z "$CHECKMORE" ]; then
  CRATES="$CRATES \
    $BASE/language/move-prover/abigen\
    $BASE/language/move-prover/docgen\
    $BASE/language/move-prover/errmapgen\
    $BASE/language/move-prover/interpreter\
    $BASE/language/move-prover/interpreter-testsuite\
    $BASE/language/move-prover/lab\
    $BASE/language/move-prover/test-utils\
    $BASE/language/tools/move-package\
    $BASE/language/tools/move-cli\
    $BASE/language/tools/move-unit-test\
  "
fi

ARTIFACT_CRATES="\
  $BASE/language/move-stdlib\
  $BASE/diem-move/diem-framework\
"

BUILD_EXPERIMENTAL="$BASE/diem-move/diem-framework"

MOVE_TEST_CRATES="\
  $BASE/language/move-stdlib\
  $BASE/diem-move/diem-framework\
"

MOVE_E2E_TEST_CRATES="\
  $BASE/language/move-lang/functional-tests\
  $BASE/language/e2e-testsuite\
  $BASE/language/tools/move-cli\
  $BASE/diem-move/df-cli\
  $BASE/language/move-stdlib\
  $BASE/diem-move/diem-framework\
"


if [ ! -z "$CHECK" ]; then
  for dir in $CRATES; do
    echo "*************** [check-pr] Checking crate $dir"
    (
      cd $dir
      if [ ! -z "$ALSO_TEST" ]; then
        cargo test $BUILD_MODE
      fi
      cargo xfmt
      cargo xclippy
      cargo xlint
    )
  done
fi

if [ ! -z "$GEN_ARTIFACTS" ]; then
  for dir in $ARTIFACT_CRATES; do
    echo "*************** [check-pr] Generating artifacts for crate $dir"
    (
      cd $dir
      cargo run $BUILD_MODE
    )
    if [[  $BUILD_EXPERIMENTAL == "$dir"  ]]; then
        echo "Building additional experimental artifact in $dir"
        (
            cd $dir
            cargo run -- --package experimental
        )
    fi
  done
fi

if [ ! -z "$GIT_CHECKS" ]; then
   echo "*************** [check-pr] Diem git checks"
   $BASE/scripts/git-checks.sh
fi

if [ ! -z "$MOVE_TESTS" ]; then
  for dir in $MOVE_TEST_CRATES; do
    echo "*************** [check-pr] Move tests $dir"
    (
      cd $dir
      cargo test $BUILD_MODE
    )
  done
fi

if [ ! -z "$MOVE_E2E_TESTS" ]; then
  for dir in $MOVE_E2E_TEST_CRATES; do
    echo "*************** [check-pr] Move e2e tests $dir"
    (
      cd $dir
      cargo test $BUILD_MODE
    )
  done
fi
