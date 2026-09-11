// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A package build writes an XIR interface per module and records a digest of
//! the set.
//!
//! Nothing consults either yet — cache validity is a later step — so these
//! tests are a measurement, and the properties worth measuring are that the
//! artifacts appear, that they are readable as interfaces, and that the digest
//! is stable across builds while tracking real API changes.

use move_package::{compilation::package_layout::CompiledPackageLayout, BuildConfig};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

const PACKAGE: &str = "tests/test_sources/compilation/basic_no_deps_test_mode";

fn build_into(dir: &Path) -> PathBuf {
    BuildConfig {
        dev_mode: true,
        test_mode: false,
        install_dir: Some(dir.to_path_buf()),
        ..Default::default()
    }
    .compile_package(Path::new(PACKAGE), &mut Vec::new())
    .unwrap();
    dir.join(CompiledPackageLayout::Root.path()).join("test")
}

fn interface_files(package_root: &Path) -> Vec<PathBuf> {
    let dir = package_root.join(CompiledPackageLayout::CompiledInterfaces.path());
    if !dir.is_dir() {
        return vec![];
    }
    let mut files = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn recorded_hash(package_root: &Path) -> Option<String> {
    let build_info =
        fs::read_to_string(package_root.join(CompiledPackageLayout::BuildInfo.path())).unwrap();
    build_info.lines().find_map(|line| {
        line.strip_prefix("interface_hash: ")
            .map(|hash| hash.trim().to_owned())
    })
}

#[test]
fn a_build_writes_an_interface_per_module_and_a_digest() {
    let dir = tempdir().unwrap();
    let package_root = build_into(dir.path());

    let interfaces = interface_files(&package_root);
    assert!(
        !interfaces.is_empty(),
        "no interfaces were written to {}",
        package_root.display()
    );

    // Each is a readable XIR interface, not just a file that exists.
    for path in &interfaces {
        let json = fs::read_to_string(path).unwrap();
        let module: move_model_exchange::XirModule = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("`{}` is not a valid interface: {e}", path.display()));
        assert!(
            module.functions.iter().all(|f| f.blocks.is_empty()),
            "an interface must not carry function bodies"
        );
    }

    assert!(
        recorded_hash(&package_root).is_some(),
        "BuildInfo.yaml records no interface_hash"
    );
}

/// Two builds of unchanged source record the same digest.
///
/// The install directories differ, so this also covers the build path leaking
/// in — the failure mode that forces `.mv`-based hashing to canonicalize.
#[test]
fn rebuilding_unchanged_sources_records_the_same_digest() {
    let first = tempdir().unwrap();
    let second = tempdir().unwrap();
    let first_hash = recorded_hash(&build_into(first.path()));
    let second_hash = recorded_hash(&build_into(second.path()));
    assert!(first_hash.is_some());
    assert_eq!(
        first_hash, second_hash,
        "the digest is not reproducible across builds"
    );
}

/// A package that cannot export an interface still builds.
///
/// This is the property that makes the feature safe to land while nothing
/// consumes it. `export_interface` rejects a module with a non-private
/// constant, and that must cost the build its interfaces — not its bytecode.
#[test]
fn a_package_that_cannot_export_still_builds() {
    let dir = tempdir().unwrap();
    let sources = dir.path().join("pkg");
    fs::create_dir_all(sources.join("sources")).unwrap();
    fs::write(
        sources.join("Move.toml"),
        "[package]\nname = \"unexportable\"\nversion = \"0.0.0\"\n\n[addresses]\nA = \"0xcafe\"\n",
    )
    .unwrap();
    // A `public const` is interface surface XIR has no table for, so the
    // exporter reports it rather than emitting a partial interface.
    fs::write(
        sources.join("sources").join("m.move"),
        "module A::m { public const SHARED: u64 = 7; public fun get(): u64 { SHARED } }\n",
    )
    .unwrap();

    let install = dir.path().join("build-out");
    BuildConfig {
        dev_mode: false,
        test_mode: false,
        install_dir: Some(install.clone()),
        ..Default::default()
    }
    .compile_package(&sources, &mut Vec::new())
    .expect("the package must still compile");

    let package_root = install
        .join(CompiledPackageLayout::Root.path())
        .join("unexportable");
    assert!(
        package_root
            .join(CompiledPackageLayout::CompiledModules.path())
            .join("m.mv")
            .exists(),
        "bytecode must still be produced"
    );
    assert!(
        interface_files(&package_root).is_empty(),
        "a package that cannot export must write no partial interface set"
    );
    assert_eq!(
        recorded_hash(&package_root),
        None,
        "and no digest, since a digest over a partial set would be misleading"
    );
}
