// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Build script that exports a marker for dependent crates.
//!
//! This enables aptos-cached-packages to detect when move-package (and its
//! compiler dependencies) have been rebuilt, triggering framework cache
//! invalidation when compiler code changes.

use std::path::PathBuf;

fn main() {
    // Write a marker file with a unique build ID to a known location.
    // When move-package rebuilds, this file gets updated with a new timestamp,
    // which aptos-cached-packages can detect to invalidate its cache.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));

    // Write the marker to target/.move_package_build_marker
    // Navigate from OUT_DIR (target/debug/build/move-package-xxx/out) to target/
    if let Some(target_dir) = out_dir.ancestors().nth(4) {
        let marker_path = target_dir.join(".move_package_build_marker");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let _ = std::fs::write(&marker_path, &timestamp);
    }
}
