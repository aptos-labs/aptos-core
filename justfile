# aptos-core development task runner
# Run `just --list` to see all available recipes.

set shell := ["/bin/sh", "-cu"]

# Default recipe: show available commands
default:
    @just --list --unsorted

# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

# Run the full dev environment setup (build tools by default)
setup *FLAGS:
    ./scripts/dev_setup.sh {{ FLAGS }}

# Minimal setup for building the CLI only
setup-minimal:
    ./scripts/cli/minimal_cli_build.sh

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

# Build a specific package (or the whole workspace if omitted)
build *ARGS:
    cargo build {{ ARGS }}

# Build with the CI-optimised profile
build-ci *ARGS:
    cargo build --profile=ci {{ ARGS }}

# Build a release binary for a package
build-release *ARGS:
    cargo build --release {{ ARGS }}

# Build with the performance profile (.cargo/performance.toml)
build-perf *ARGS:
    cargo build-perf {{ ARGS }}

# ---------------------------------------------------------------------------
# Check & Compile
# ---------------------------------------------------------------------------

# Quick compilation check (no codegen) for a package
check *ARGS:
    cargo check {{ ARGS }}

# Check the entire workspace
check-all:
    cargo check --workspace --all-targets

# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------

# Run tests for a specific package
test *ARGS:
    cargo test {{ ARGS }}

# Run tests with nextest (if installed)
nextest *ARGS:
    cargo nextest run {{ ARGS }}

# Run a quarantined/flaky test with retries
test-quarantined crate retries="3":
    ./scripts/run_quarantined.sh -f -c {{ crate }} -r {{ retries }}

# ---------------------------------------------------------------------------
# Lint & Format
# ---------------------------------------------------------------------------

# Run the full lint suite (clippy + fmt + sort + machete)
lint:
    ./scripts/rust_lint.sh

# Run the full lint suite in check mode (CI-safe, no modifications)
lint-check:
    ./scripts/rust_lint.sh --check

# Run clippy with the aptos-core configuration
clippy:
    cargo xclippy

# Format code with nightly rustfmt
fmt:
    cargo +nightly fmt

# Check formatting without modifying files
fmt-check:
    cargo +nightly fmt --check

# Sort Cargo.toml dependencies
sort:
    cargo sort --grouped --workspace

# Check Cargo.toml dependency sort order
sort-check:
    cargo sort --grouped --workspace --check

# Detect unused dependencies
machete:
    cargo machete

# ---------------------------------------------------------------------------
# Framework & Code Generation
# ---------------------------------------------------------------------------

# Rebuild cached Move framework artifacts (head.mrb + SDK builders)
cached-packages:
    ./scripts/cargo_build_aptos_cached_packages.sh

# Verify cached framework artifacts are up-to-date (CI)
cached-packages-check:
    ./scripts/cargo_build_aptos_cached_packages.sh --check

# Regenerate OpenAPI specs for the Aptos Node API
openapi:
    cargo run -p aptos-openapi-spec-generator -- -f yaml -o api/doc/spec.yaml
    cargo run -p aptos-openapi-spec-generator -- -f json -o api/doc/spec.json

# Regenerate protobuf definitions
protos:
    cd protos && ./scripts/build_protos.sh

# Regenerate serde-reflection format files and OpenAPI specs
regenerate:
    ./scripts/authenticator_regenerate.sh

# Build Move documentation
move-docs *OUTDIR:
    ./scripts/move_docs.sh {{ OUTDIR }}

# ---------------------------------------------------------------------------
# Git & Pre-submit
# ---------------------------------------------------------------------------

# Run git hygiene checks (no merges, no submodules, no bad filenames)
git-checks:
    ./scripts/git-checks.sh

# Fail if there are modified tracked files (useful in CI)
check-clean:
    ./scripts/fail_if_modified_files.sh

# Full pre-submit: lint-check + cached-packages-check + check-clean
presubmit: lint-check cached-packages-check check-clean

# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

# Build the CLI in its release profile
cli-build:
    cargo build -p aptos --profile cli

# Install a released CLI version from git (e.g. `just cli-install 4.5.0`)
cli-install version:
    ./scripts/cli/cargo_install_cli.sh {{ version }}

# ---------------------------------------------------------------------------
# PGO (Profile-Guided Optimisation)
# ---------------------------------------------------------------------------

# Generate PGO profile data
pgo-profile output:
    ./scripts/pgo.sh profile {{ output }}

# Build with PGO profile data
pgo-build profile *ARGS:
    ./scripts/pgo.sh build {{ profile }} {{ ARGS }}

# Run with PGO profile data
pgo-run profile *ARGS:
    ./scripts/pgo.sh run {{ profile }} {{ ARGS }}

# ---------------------------------------------------------------------------
# API (OpenAPI spec tooling in api/)
# ---------------------------------------------------------------------------

# Lint the OpenAPI spec
api-lint:
    make -C api lint

# Serve the API docs locally on port 8888
api-serve:
    make -C api serve

# ---------------------------------------------------------------------------
# Dependency management
# ---------------------------------------------------------------------------

# Update semver-compatible dependency versions
update-deps:
    ./scripts/cargo_update_outdated.sh

# Show the dependency tree for a package
deps pkg:
    cargo tree -p {{ pkg }}

# Show reverse dependencies (what depends on a package)
rdeps pkg:
    cargo tree -p {{ pkg }} --invert
