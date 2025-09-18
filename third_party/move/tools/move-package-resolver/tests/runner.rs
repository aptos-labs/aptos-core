// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_package_cache::{DebugPackageCacheListener, PackageCache};
use move_package_resolver::{graph_to_mermaid, resolve, PackageLock};
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use std::{fmt::Write, path::Path};

pub fn run_resolver_expected_output_tests(manifest_path: &Path) -> datatest_stable::Result<()> {
    let package_path = manifest_path.parent().unwrap();

    let crate_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let package_cache =
        PackageCache::new_with_listener(crate_path.join("cache"), DebugPackageCacheListener)?;
    let package_lock_path = package_path.join("Move.lock");
    let mut package_lock = PackageLock::load_from_file_or_empty(&package_lock_path)?;

    let res = tokio::runtime::Runtime::new()?
        .block_on(async { resolve(&package_cache, &mut package_lock, package_path, true).await });

    let mut output = String::new();
    match res {
        Ok(graph) => {
            let mermaid = graph_to_mermaid(&graph, Some(crate_path));
            writeln!(output, "success")?;
            writeln!(output)?;
            writeln!(output, "```mermaid")?;
            writeln!(output, "{}", mermaid)?;
            writeln!(output, "```")?;
        },
        Err(err) => {
            writeln!(output, "error")?;
            writeln!(output)?;
            writeln!(output, "{}", err)?;
        },
    }
    output = output.replace(&format!("{}/", crate_path.display()), "");

    let toml = toml::to_string_pretty(&package_lock)?;

    verify_or_update_baseline(&package_lock_path, &toml)?;
    verify_or_update_baseline(&package_path.join("output.md"), &output)?;

    Ok(())
}
