#!/bin/bash

# To run all tests that may depend on the compiler, run this script.
# To regenerate test file outputs, set environment variable UB=1 when running.

# Be careful to examine all diffs before committing to a PR.
# Note that there may be some issues with test interference,
# as dependences between tests may not be propertly declared and/or honored.

# You should search the output for "error: test failed" to see if any tests fail,
#
# To see what the state of MOVE_COMPILER_V2 was for that failed run, use the
# following command:
#    grep 'MOVE_COMPILER_V2\|error: test failed' run-all.out* | fgrep -B1 error
#

set -x

unset RUST_BACKTRACE
export RUST_MIN_STACK=4297152

MOVE_COMPILER_V2=false \
    cargo test --locked --no-fail-fast \
    -p "move-compiler" \
    -p "move-compiler-transactional-tests" \
    -p "move-compiler-v2" \
    -p "move-compiler-v2-transactional-tests" \
    -p "move-prover"

for value in false true; do
    echo "MOVE_COMPILER_V2=$value "
    MOVE_COMPILER_V2=$value\
        cargo test --locked --no-fail-fast \
        -p "aptos-transactional-test-harness" \
        -p "bytecode-verifier-transactional-tests" \
        -p "move-async-vm" \
        -p "move-cli" \
        -p "move-model" \
        -p "move-package" \
        -p "move-prover-bytecode-pipeline" \
        -p "move-stackless-bytecode" \
        -p "move-to-yul" \
        -p "move-transactional-test-runner" \
        -p "move-unit-test" \
        -p "move-vm-transactional-tests"
    
    # these tests do not produce .exp files,
    # so don't capture hidden outputs
    echo "MOVE_COMPILER_V2=$value "
    MOVE_COMPILER_V2=$value \
        cargo test --locked --no-fail-fast \
        -p "aptos-move-stdlib" \
        -p "move-abigen" \
        -p "move-docgen" \
        -p "move-stdlib" \
        -p "move-table-extension" -- --nocapture

    echo "MOVE_COMPILER_V2=$value "
    MOVE_COMPILER_V2=$value \
        cargo test --locked --no-fail-fast \
        -p "aptos-api" \
        -p "e2e-move-tests" \
        -p "aptos-framework" \
        -p "move-vm-integration-tests" \
        -p "aptos-move-examples" -- --nocapture
done

# Relevant packages run along with their directories, for convenient reference:
# 
# ./api/Cargo.toml:name = "aptos-api"
# ./aptos-move/aptos-transactional-test-harness/Cargo.toml:name = "aptos-transactional-test-harness"
# ./aptos-move/e2e-move-tests/Cargo.toml:name = "e2e-move-tests"
# ./aptos-move/framework/Cargo.toml:name = "aptos-framework"
# ./aptos-move/framework/move-stdlib/Cargo.toml:name = "aptos-move-stdlib"
# ./aptos-move/move-examples/Cargo.toml:name = "aptos-move-examples"
# ./third_party/move/evm/move-to-yul/Cargo.toml:name = "move-to-yul"
# ./third_party/move/extensions/async/move-async-vm/Cargo.toml:name = "move-async-vm"
# ./third_party/move/extensions/move-table-extension/Cargo.toml:name = "move-table-extension"
# ./third_party/move/move-bytecode-verifier/transactional-tests/Cargo.toml:name = "bytecode-verifier-transactional-tests"
# ./third_party/move/move-compiler-v2/Cargo.toml:name = "move-compiler-v2"
# ./third_party/move/move-compiler-v2/transactional-tests/Cargo.toml:name = "move-compiler-v2-transactional-tests"
# ./third_party/move/move-compiler/Cargo.toml:name = "move-compiler"
# ./third_party/move/move-compiler/transactional-tests/Cargo.toml:name = "move-compiler-transactional-tests"
# ./third_party/move/move-model/Cargo.toml:name = "move-model"
# ./third_party/move/move-model/bytecode/Cargo.toml:name = "move-stackless-bytecode"
# ./third_party/move/move-prover/Cargo.toml:name = "move-prover"
# ./third_party/move/move-prover/bytecode-pipeline/Cargo.toml:name = "move-prover-bytecode-pipeline"
# ./third_party/move/move-prover/move-abigen/Cargo.toml:name = "move-abigen"
# ./third_party/move/move-prover/move-docgen/Cargo.toml:name = "move-docgen"
# ./third_party/move/move-stdlib/Cargo.toml:name = "move-stdlib"
# ./third_party/move/move-vm/integration-tests/Cargo.toml:name = "move-vm-integration-tests"
# ./third_party/move/move-vm/transactional-tests/Cargo.toml:name = "move-vm-transactional-tests"
# ./third_party/move/testing-infra/transactional-test-runner/Cargo.toml:name = "move-transactional-test-runner"
# ./third_party/move/tools/move-cli/Cargo.toml:name = "move-cli"
# ./third_party/move/tools/move-package/Cargo.toml:name = "move-package"
# ./third_party/move/tools/move-unit-test/Cargo.toml:name = "move-unit-test"
