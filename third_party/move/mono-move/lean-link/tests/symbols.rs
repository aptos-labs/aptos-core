// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The staticlib artifact must export the versioned C ABI. This is the
//! contract the C shim links against, so a lost export fails here instead
//! of at Lean link time.

use std::{path::PathBuf, process::Command};

const EXPORTS: &[&str] = &[
    "leaner_monovm_abi_version",
    "leaner_monovm_run",
    "leaner_monovm_buffer_free",
];

#[test]
fn the_staticlib_exports_the_c_abi() {
    let cargo = env!("CARGO");
    let status = Command::new(cargo)
        .args(["build", "-p", "mono-move-lean-link"])
        .status()
        .expect("cargo must be runnable");
    assert!(status.success(), "building the adapter staticlib failed");

    let archive = target_dir().join("debug").join("libmono_move_lean_link.a");
    assert!(
        archive.exists(),
        "staticlib artifact {archive:?} is missing"
    );

    let output = Command::new("nm")
        .arg("-g")
        .arg(&archive)
        .output()
        .expect("nm must be installed to inspect the staticlib");
    assert!(output.status.success(), "nm failed on {archive:?}");
    let symbols = String::from_utf8_lossy(&output.stdout);
    for export in EXPORTS {
        assert!(
            symbols.contains(export),
            "staticlib does not export {export}; nm output:\n{symbols}"
        );
    }
}

/// Resolves the workspace target directory: `CARGO_TARGET_DIR` when set,
/// otherwise `target/` under the workspace root found by walking up from
/// this crate's manifest directory.
fn target_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").exists() && is_workspace_root(dir))
        .map(|root| root.join("target"))
        .unwrap_or_else(|| manifest.join("target"))
}

fn is_workspace_root(dir: &std::path::Path) -> bool {
    let Ok(manifest) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
        return false;
    };
    manifest.starts_with("[workspace]") || manifest.lines().any(|line| line.trim() == "[workspace]")
}
