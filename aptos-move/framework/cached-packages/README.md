# aptos-cached-packages

This crate provides pre-compiled Move framework packages as checked-in artifacts so that other crates can depend on them without rebuilding the Move framework from source every time.

## Artifacts

The following files are **generated** and checked into the repo:

- `src/head.mrb` — BCS-serialized `ReleaseBundle` containing all compiled Move framework packages (move-stdlib, aptos-stdlib, aptos-framework, aptos-token, aptos-token-objects, aptos-trading, aptos-experimental).
- `src/aptos_framework_sdk_builder.rs` — Rust SDK bindings for aptos-framework.
- `src/aptos_token_sdk_builder.rs` — Rust SDK bindings for aptos-token.
- `src/aptos_token_objects_sdk_builder.rs` — Rust SDK bindings for aptos-token-objects.

## Updating the artifacts

These artifacts must be regenerated whenever you change something that affects the compiled output. This includes:

- Move source files under `aptos-move/framework/` (e.g. in `aptos-framework/sources/`, `aptos-stdlib/sources/`, `move-stdlib/sources/`, etc.)
- Move compiler implementation or options (crates under `third_party/move/`)
- Build options in `aptos-move/framework/src/aptos.rs` (`create_release_options`)

To regenerate, run from anywhere in the repo:

```bash
scripts/cargo_build_aptos_cached_packages.sh
```

Then commit the updated artifacts together with your changes.

CI runs the same script with `--check` to verify the artifacts are fresh. This check is **advisory (non-blocking)** because concurrent PR merges can make the checked-in artifacts appear stale even when a PR itself is correct. If the check fails on your PR, run the command above and commit the result.

A nightly workflow (`nightly-update-cached-packages.yaml`) automatically rebuilds the artifacts against `main`, opens an auto-merging PR if anything changed, and notifies Slack if CI fails on that PR.
