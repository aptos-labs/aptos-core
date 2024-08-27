#!/bin/bash
# Copyright (c) Aptos Foundation
# Copyright (c) The Move Contributors
# SPDX-License-Identifier: Apache-2.0

# A script to check whether a local commit related to Move is ready for a PR.

# Note that if tests aren't running for you try `cargo update` and maybe
# `cargo install cargo-nextest`.

set -e

MOVE_PR_PROFILE="${MOVE_PR_PROFILE:-ci}"
MOVE_PR_NEXTEST_PROFILE="${MOVE_PR_NEXTEST_PROFILE:-smoke-test}"

BASE=$(git rev-parse --show-toplevel)

# This is currently setup for the aptos-core environment. If move is at a different
# location, this need to be changed.
MOVE_BASE=$BASE/third_party/move

echo "*************** [move-pr] Assuming move root at $MOVE_BASE"

# Run only tests which would also be run on CI
export ENV_TEST_ON_CI=1

while getopts "htcgdi2a" opt; do
  case $opt in
    h)
      cat <<EOF
Performs CI equivalent checks on a local client
Usage:
    check_pr <flags>
Flags:
    -h   Print this help
    -t   Run tests
    -i   In addition to -t, run integration tests (Aptos framework and e2e tests)
    -2   Run integration tests with the v2 compiler
    -c   Run xclippy and fmt +nightly
    -g   Run the git checks script (whitespace check). This works
         only for committed clients.
    -d   Run artifact generation for move-stdlib and other Move libraries.
    -a   All of the above
    With no options script behaves like -tcg is given.
    You can use the `MOVE_PR_PROFILE` environment variable to
    determine which cargo profile to use. (The default
    is `ci`, `debug` might be faster for build.)
    You can also run those tests with `UB=1` to record new
    baseline files where applicable.
EOF
      exit 1
      ;;
    t)
      TEST=1
      ;;
    i)
      INTEGRATION_TEST=1
      ;;
    2)
      COMPILER_V2_TEST=1
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
    a)
      INTEGRATION_TEST=1
      COMPILER_V2_TEST=1
      GEN_ARTIFACTS=1
      GIT_CHECKS=1
  esac
done

if [ "$OPTIND" -eq 1 ]; then
  TEST=1
  CHECK=1
  GIT_CHECKS=1
fi

ARTIFACT_CRATE_PATHS="\
  move-stdlib\
"

# This is a partial list of Move crates, to keep this script fast.
# May be extended as needed but should be kept minimal.
MOVE_CRATES="\
  -p move-stackless-bytecode\
  -p move-stdlib\
  -p move-bytecode-verifier\
  -p move-binary-format\
  -p move-compiler\
  -p move-compiler-transactional-tests\
  -p move-compiler-v2\
  -p move-compiler-v2-transactional-tests\
  -p move-ir-compiler-transactional-tests\
  -p move-prover-boogie-backend\
  -p move-prover\
  -p move-transactional-test-runner\
  -p move-vm-runtime\
  -p move-vm-types\
"

# This is a list of crates for integration testing which depends on the
# MOVE_COMPILER_V2 env var.
MOVE_CRATES_V2_ENV_DEPENDENT="\
  -p aptos-transactional-test-harness \
  -p bytecode-verifier-tests \
  -p bytecode-verifier-transactional-tests \
  -p move-async-vm \
  -p move-cli \
  -p move-model \
  -p move-package \
  -p move-prover-bytecode-pipeline \
  -p move-stackless-bytecode \
  -p move-to-yul \
  -p move-transactional-test-runner \
  -p move-unit-test \
  -p move-vm-transactional-tests \
  -p aptos-move-stdlib\
  -p move-abigen\
  -p move-docgen\
  -p move-stdlib\
  -p move-table-extension\
  -p move-vm-integration-tests\
  -p aptos-move-examples\
  -p e2e-move-tests\
  -p aptos-framework\
"

# Crates which do depend on compiler env but currently
# do not maintain separate v2 baseline files. Those
# are listed here for documentation and later fixing.
MOVE_CRATES_V2_ENV_DEPENDENT_FAILURES="\
  -p aptos-api\
"

if [ ! -z "$CHECK" ]; then
  echo "*************** [move-pr] Running checks"
  (
    cd $BASE
    cargo xclippy
    cargo +nightly fmt
    cargo sort --grouped --workspace
  )
fi

CARGO_OP_PARAMS="--profile $MOVE_PR_PROFILE"
CARGO_NEXTEST_PARAMS="--profile $MOVE_PR_NEXTEST_PROFILE --cargo-profile $MOVE_PR_PROFILE $MOVE_PR_NEXTEST_ARGS"

# Artifact generation needs to be run before testing as tests may depend on its result
if [ ! -z "$GEN_ARTIFACTS" ]; then
    for dir in $ARTIFACT_CRATE_PATHS; do
        echo "*************** [move-pr] Generating artifacts for crate $dir"
        (
            cd $MOVE_BASE/$dir
            cargo run $CARGO_OP_PARAMS
        )
    done

    # Add hoc treatment
    (
        cd $BASE
        cargo build $CARGO_OP_PARAMS -p aptos-cached-packages
    )
fi

if [ ! -z "$TEST" ]; then
  echo "*************** [move-pr] Running tests"
  (
    # It is important to run all tests from one cargo command to keep cargo features
    # stable.
    cd $BASE
    cargo nextest run $CARGO_NEXTEST_PARAMS \
     $MOVE_CRATES
  )
fi

if [ ! -z "$INTEGRATION_TEST" ]; then
  echo "*************** [move-pr] Running integration tests"
  (
    cd $BASE
    MOVE_COMPILER_V2=false cargo build $CARGO_OP_PARAMS \
       $MOVE_CRATES $MOVE_CRATES_V2_ENV_DEPENDENT
    MOVE_COMPILER_V2=false cargo nextest run $CARGO_NEXTEST_PARAMS \
       $MOVE_CRATES $MOVE_CRATES_V2_ENV_DEPENDENT
  )
fi

if [ ! -z "$COMPILER_V2_TEST" ]; then
  echo "*************** [move-pr] Running integration tests with compiler v2"
  (
    cd $BASE
    MVC_DOCGEN_OUTPUT_DIR=tests/compiler-v2-doc MOVE_COMPILER_V2=true cargo build $CARGO_OP_PARAMS \
       $MOVE_CRATES_V2_ENV_DEPENDENT
    MVC_DOCGEN_OUTPUT_DIR=tests/compiler-v2-doc \
       MOVE_COMPILER_V2=true cargo nextest run $CARGO_NEXTEST_PARAMS \
       $MOVE_CRATES_V2_ENV_DEPENDENT
  )
fi

if [ ! -z "$GIT_CHECKS" ]; then
   echo "*************** [move-pr] Running git checks"
   $BASE/scripts/git-checks.sh
fi
