// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end: generate a (move-stdlib-only) bundle from the synthetic config,
//! verify it, and check the expected file layout.

use aptos_release_tool::{bundle, commands, init_core_path};
use std::path::PathBuf;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

/// Runs on a dedicated large-stack thread: the Move framework compiler recurses
/// deeply and overflows a default test-thread stack (the real CLI gets the room
/// from the process main thread).
#[test]
fn generate_then_verify_bundle() {
    const STACK_SIZE: usize = 256 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build runtime")
                .block_on(generate_then_verify_bundle_inner());
        })
        .expect("spawn test thread")
        .join()
        .expect("test thread panicked");
}

async fn generate_then_verify_bundle_inner() {
    let core_path = init_core_path();

    let config_path = test_data_dir().join("config.yaml");
    let tmp = tempfile::tempdir().expect("tempdir");
    let bundle_path = tmp.path().join("bundle");

    // `generate` self-verifies (checksums, digest, layout) before returning, so
    // no need to call `verify` manually.
    commands::generate::run(&config_path, &bundle_path, &core_path)
        .await
        .expect("generate bundle");

    // The deterministic, framework-independent files are all present.
    for rel in [
        bundle::BUNDLE_TOML,
        bundle::CONFIG_YAML,
        bundle::METADATA_JSON,
        "gas/old.json",
        "gas/new.json",
        "summary/gas-schedule-changes.md",
        "summary/feature-flags.md",
    ] {
        assert!(bundle_path.join(rel).is_file(), "missing {}", rel);
    }
    let has_move = std::fs::read_dir(bundle_path.join(bundle::SCRIPTS_DIR))
        .expect("read scripts dir")
        .filter_map(Result::ok)
        .any(|e| e.path().extension().is_some_and(|x| x == "move"));
    assert!(has_move, "no .move scripts were generated");
}
