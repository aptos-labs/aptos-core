// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Modular compilation must be indistinguishable from the monolithic path.
//!
//! The monolithic build is the reference, and the only claim worth making about
//! the modular one is that it produces the same bytecode. Everything else it
//! does — per-package artifacts, interface hashes, caching — is machinery in
//! service of that, and none of it is worth having if the output differs.

use move_package::{compilation::package_layout::CompiledPackageLayout, BuildConfig};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

/// A three-package chain: `root` → `mid` → `leaf`.
///
/// Nothing here calls a cross-package `public inline` function, so the modular
/// path can actually engage. The fallback case is covered separately.
fn write_chain(dir: &Path, mid_body: &str, root_body: &str) {
    let package = |name: &str, address: &str, deps: &str, source: &str| {
        let path = dir.join(name.to_lowercase());
        fs::create_dir_all(path.join("sources")).unwrap();
        fs::write(
            path.join("Move.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n\n\
                 [addresses]\n{address} = \"0xcafe\"\n\n{deps}"
            ),
        )
        .unwrap();
        fs::write(path.join("sources").join("m.move"), source).unwrap();
    };

    package("Leaf", "L", "", LEAF);
    package(
        "Mid",
        "M",
        "[dependencies]\nLeaf = { local = \"../leaf\" }\n",
        mid_body,
    );
    package(
        "Root",
        "R",
        "[dependencies]\nMid = { local = \"../mid\" }\n",
        root_body,
    );
}

const LEAF: &str = r#"
module L::leaf {
    public struct Token has copy, drop, store { v: u64 }
    public fun make(v: u64): Token { Token { v } }
    public fun read(t: &Token): u64 { t.v }
}
"#;

const MID: &str = r#"
module M::mid {
    use L::leaf;
    public fun doubled(v: u64): u64 {
        let t = leaf::make(v);
        leaf::read(&t) * 2
    }
}
"#;

const ROOT: &str = r#"
module R::root {
    use M::mid;
    use L::leaf;
    public fun go(): u64 {
        // Packed directly rather than via `leaf::make`, so the compiler has to
        // synthesize `pack$Token` and `borrow$Token$0` handles from the
        // interface's struct declaration — the interface carries no such
        // functions.
        let t = leaf::Token { v: 3 };
        mid::doubled(t.v) + leaf::read(&t)
    }
}
"#;

/// Compiles `root` and returns every produced module's bytecode by name.
fn compile(chain: &Path, install: &Path, modular: bool) -> BTreeMap<String, Vec<u8>> {
    compile_logged(chain, install, modular).0
}

/// As [`compile`], also returning the build log, which names each package as it
/// is compiled or reused.
fn compile_logged(
    chain: &Path,
    install: &Path,
    modular: bool,
) -> (BTreeMap<String, Vec<u8>>, String) {
    let mut log = Vec::new();
    let package = BuildConfig {
        dev_mode: false,
        test_mode: false,
        install_dir: Some(install.to_path_buf()),
        modular_compilation: modular,
        ..Default::default()
    }
    .compile_package(&chain.join("root"), &mut log)
    .unwrap_or_else(|e| panic!("modular={modular} build failed: {e:?}"));

    let modules = package
        .root_compiled_units
        .iter()
        .map(|unit| (String::new(), unit))
        .chain(
            package
                .deps_compiled_units
                .iter()
                .map(|(_, u)| (String::new(), u)),
        )
        .map(|(_, unit)| (unit.unit.name().to_string(), unit.unit.serialize(None)))
        .collect();
    (modules, String::from_utf8_lossy(&log).into_owned())
}

/// Packages the log says were reused rather than recompiled.
fn cached_packages(log: &str) -> Vec<String> {
    let mut names = log
        .lines()
        // The log is colorized; match on the payload rather than the exact
        // escape sequences.
        .filter(|line| line.contains("CACHED"))
        .map(|line| {
            line.split_whitespace()
                .last()
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn chain_in(dir: &TempDir, mid: &str, root: &str) -> PathBuf {
    let chain = dir.path().join("chain");
    write_chain(&chain, mid, root);
    chain
}

/// The headline property.
#[test]
fn modular_and_monolithic_builds_agree() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);

    let monolithic = compile(&chain, &dir.path().join("mono"), false);
    let modular = compile(&chain, &dir.path().join("modular"), true);

    assert!(
        monolithic.len() >= 3,
        "expected all three modules, got {:?}",
        monolithic.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        monolithic.keys().collect::<Vec<_>>(),
        modular.keys().collect::<Vec<_>>(),
        "the two paths produced different modules"
    );
    for (name, bytes) in &monolithic {
        assert_eq!(
            bytes,
            modular.get(name).unwrap(),
            "`{name}` differs between the monolithic and modular builds"
        );
    }
}

/// And the modular path really did compile packages separately, rather than
/// silently falling back everywhere and passing the test above trivially.
#[test]
fn each_package_is_built_on_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);
    let install = dir.path().join("modular");
    compile(&chain, &install, true);

    let build_root = install.join(CompiledPackageLayout::Root.path());
    for package in ["Leaf", "Mid", "Root"] {
        let interfaces = build_root
            .join(package)
            .join(CompiledPackageLayout::CompiledInterfaces.path());
        assert!(
            interfaces.is_dir() && fs::read_dir(&interfaces).unwrap().count() > 0,
            "`{package}` has no interfaces at {}, so it was not built on its own",
            interfaces.display()
        );
    }
}

/// Rebuilding an untouched graph recompiles nothing.
#[test]
fn an_unchanged_rebuild_reuses_every_package() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);
    let install = dir.path().join("modular");

    let (first, _) = compile_logged(&chain, &install, true);
    let (second, log) = compile_logged(&chain, &install, true);

    assert_eq!(
        cached_packages(&log),
        vec!["Leaf", "Mid", "Root"],
        "an unchanged rebuild should reuse everything; log was:\n{log}"
    );
    assert_eq!(
        first, second,
        "a cached rebuild produced different bytecode"
    );
}

/// Editing a *body* in the leaf leaves its dependents cached.
///
/// This is the property the interface hash exists for, and the one that
/// separates it from hashing sources: `Leaf` must rebuild because its source
/// changed, but its API did not move, so `Mid` and `Root` have nothing to
/// recompile. Hashing dependency sources instead would rebuild all three.
#[test]
fn a_body_only_change_does_not_rebuild_dependents() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);
    let install = dir.path().join("modular");
    compile_logged(&chain, &install, true);

    // Same signatures, different implementation.
    fs::write(
        chain.join("leaf").join("sources").join("m.move"),
        LEAF.replace(
            "public fun read(t: &Token): u64 { t.v }",
            "public fun read(t: &Token): u64 { let x = t.v; x }",
        ),
    )
    .unwrap();

    let (_, log) = compile_logged(&chain, &install, true);
    assert_eq!(
        cached_packages(&log),
        vec!["Mid", "Root"],
        "a body-only change should rebuild only the edited package; log was:\n{log}"
    );
}

/// Editing the leaf's *API* rebuilds everything downstream of it.
///
/// The complement of the test above: a cache that never invalidates would pass
/// that one and be catastrophically wrong here.
#[test]
fn an_api_change_rebuilds_dependents() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);
    let install = dir.path().join("modular");
    compile_logged(&chain, &install, true);

    fs::write(
        chain.join("leaf").join("sources").join("m.move"),
        LEAF.replace(
            "public fun make(v: u64): Token { Token { v } }",
            "public fun make(v: u64): Token { Token { v } }\n    public fun extra(): u64 { 9 }",
        ),
    )
    .unwrap();

    let (_, log) = compile_logged(&chain, &install, true);
    assert!(
        cached_packages(&log).is_empty(),
        "an API change must rebuild every dependent; log was:\n{log}"
    );
}

/// A package that calls a cross-package `public inline` function falls back,
/// and the fallback is invisible in the output.
///
/// This is not an edge case: `move-stdlib` exports 36 `public inline`
/// functions and `aptos-stdlib` 32, so most real graphs hit it. What must hold
/// is that falling back costs time, not correctness.
#[test]
fn a_cross_package_inline_call_falls_back_and_still_agrees() {
    let dir = tempfile::tempdir().unwrap();
    let mid = r#"
module M::mid {
    use L::leaf;
    public inline fun bump(x: u64): u64 { x + 1 }
    public fun doubled(v: u64): u64 {
        let t = leaf::make(v);
        leaf::read(&t) * 2
    }
}
"#;
    let root = r#"
module R::root {
    use M::mid;
    public fun go(): u64 {
        // Calls `mid`'s inline function across a package boundary, which an
        // interface cannot describe.
        mid::bump(mid::doubled(2))
    }
}
"#;
    let chain = chain_in(&dir, mid, root);

    let monolithic = compile(&chain, &dir.path().join("mono"), false);
    let modular = compile(&chain, &dir.path().join("modular"), true);
    assert_eq!(
        monolithic, modular,
        "the fallback changed the produced bytecode"
    );

    // `Root` fell back, so it has no interfaces of its own to offer; `Leaf`
    // still exports normally.
    let build_root = dir
        .path()
        .join("modular")
        .join(CompiledPackageLayout::Root.path());
    let leaf_interfaces = build_root
        .join("Leaf")
        .join(CompiledPackageLayout::CompiledInterfaces.path());
    assert!(
        leaf_interfaces.is_dir(),
        "a package unaffected by the fallback should still export"
    );
}
