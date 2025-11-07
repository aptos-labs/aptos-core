#!/bin/sh
set -e

# Script to automate building and running Rust binaries with PGO.
#
# Example usage:
#   ./pgo.sh profile a.profdata
#   ./pgo.sh run a.profdata -p aptos-vm-profiling --bin run-aptos-p2p
#                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                           You can replace this with arbitrary cargo run args
# 
# Build without running:
#   ./pgo.sh build a.profdata -p aptos-vm-profiling --bin run-aptos-p2p

# Shared Rust flags -- this should be in sync with the ones defined in .cargo/config.toml
RUSTFLAGS="
  --cfg tokio_unstable
  -C link-arg=-fuse-ld=lld
  -C force-frame-pointers=yes
  -C force-unwind-tables=yes
  -C target-cpu=x86-64-v3
"

# Show script usage
usage() {
    cat <<EOF
Usage: $0 profile <profile-data-path>

       $0 build <profile-data-path> [-- <cargo build args>]

       $0 run <profile-data-path> [-- <cargo run args>]
EOF
    exit 1
}

# Get the first argument as subcommand
if [ $# -lt 1 ]; then
  usage
fi
CMD="$1"
shift || true

case "$CMD" in
    profile)
        PROFILE_DATA_PATH="$1"
        [ -z "$PROFILE_DATA_PATH" ] && usage
        shift

        # Create a temporary directory for storing raw profile data
        TMPDIR=$(mktemp -d /tmp/pgo-data.XXXXXX)

        # Build the test binary with instrumentation
        # Current workload is fixed -- 1000 p2p transactions in a single block
        # Should switch to something more comprehensive once we have it.
        env RUST_BACKTRACE=1 RUST_MIN_STACK=104857600 \
            RUSTFLAGS="$RUSTFLAGS -C profile-generate=$TMPDIR" \
            cargo build --profile release -p aptos-vm-profiling --bin run-aptos-p2p

        # Run the test binary
        #
        # Note: We first compile the binary, clear the profile directory, and then run the binary.
        #       This is a defensive measure to ensure the profile data is not tainted by 
        #       build scripts.
        rm -rf "$TMPDIR/*"
        REPO_ROOT=$(dirname $(cargo locate-project --workspace --message-format plain))
        "$REPO_ROOT/target/release/run-aptos-p2p"

        # Merge the raw profile data
        # Note: A relatively up-to-date version of llvm is required. 
        #       (Rust 1.89 uses llvm-20)
        #
        #       Follow the instructions here to install:
        #         - https://apt.llvm.org/
        llvm-profdata-20 merge -o "$PROFILE_DATA_PATH" "$TMPDIR"

        # Clean up the raw profile data
        rm -rf "$TMPDIR"
        ;;
    build)
        PROFILE_DATA_PATH="$1"
        [ -z "$PROFILE_DATA_PATH" ] && usage
        shift
        PROFILE_DATA_PATH=$(realpath "$PROFILE_DATA_PATH")

        env RUST_BACKTRACE=1 RUST_MIN_STACK=104857600 \
            RUSTFLAGS="$RUSTFLAGS -C profile-use=$PROFILE_DATA_PATH" \
            cargo build --profile release "$@"
        ;;
    run)
        PROFILE_DATA_PATH="$1"
        [ -z "$PROFILE_DATA_PATH" ] && usage
        shift
        PROFILE_DATA_PATH=$(realpath "$PROFILE_DATA_PATH")

        env RUST_BACKTRACE=1 RUST_MIN_STACK=104857600 \
            RUSTFLAGS="$RUSTFLAGS -C profile-use=$PROFILE_DATA_PATH" \
            cargo run --profile release "$@"
        ;;
    *)
        usage
        ;;
esac
