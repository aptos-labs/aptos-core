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

/// A dependency artifact that no longer exists must not survive a refresh.
///
/// When the root is cached but a dependency underneath it was rebuilt, only the
/// root's *copies* of that dependency's units are refreshed. Writing over them
/// leaves behind anything since deleted or renamed, and that residue is not
/// inert: `get_compiled_units_paths` walks these directories with `walkdir`, so
/// a stale `.mv` is loaded back as part of the package on the next cache hit.
///
/// Reachable because a dependency's scripts are not covered by its interface
/// hash — deleting one rebuilds that dependency while every dependent stays
/// cache-valid, which is exactly this path.
#[test]
fn a_refresh_removes_dependency_artifacts_that_no_longer_exist() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);
    let install = dir.path().join("modular");
    compile_logged(&chain, &install, true);

    // A unit leaves three artifacts behind, and `refresh_dependency_units`
    // rewrote none of them — so a deleted or renamed unit survives as a
    // complete, decodable set. Anything less than the full triple fails to
    // load instead, which turns into a cache miss and a rebuild; it is the
    // *complete* residue that is silent, so that is what this plants.
    let root_build = install.join("build").join("Root");
    let leaf_dir = |category: &str| root_build.join(category).join("dependencies").join("Leaf");
    let ghosts = [
        ("bytecode_modules", "mv"),
        ("source_maps", "mvsm"),
        ("sources", "move"),
    ]
    .map(|(category, extension)| {
        let live = leaf_dir(category).join(format!("leaf.{extension}"));
        assert!(
            live.is_file(),
            "expected the root to carry Leaf's {category} at {}",
            live.display()
        );
        let ghost = leaf_dir(category).join(format!("ghost.{extension}"));
        fs::copy(&live, &ghost).unwrap();
        (live, ghost)
    });

    // A body-only edit rebuilds `Leaf` while `Root` stays cached — the
    // condition for refreshing rather than re-saving.
    fs::write(
        chain.join("leaf").join("sources").join("m.move"),
        LEAF.replace(
            "public fun read(t: &Token): u64 { t.v }",
            "public fun read(t: &Token): u64 { let x = t.v; x }",
        ),
    )
    .unwrap();
    let (_, log) = compile_logged(&chain, &install, true);
    assert!(
        cached_packages(&log).contains(&"Root".to_owned()),
        "this test only exercises the fix when the root is cached; log was:\n{log}"
    );

    for (live, ghost) in &ghosts {
        assert!(
            !ghost.exists(),
            "a dependency artifact that no longer exists survived the refresh: {}",
            ghost.display()
        );
        assert!(
            live.is_file(),
            "the refresh must still write the current units: {}",
            live.display()
        );
    }
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

/// A cross-package `public inline` call is compiled *through* the interface.
///
/// This used to be the fallback case, and it is the one that decides whether
/// the feature engages at all: `move-stdlib` exports 36 non-private inline
/// functions and every framework package depends on it, so a graph that fell
/// back here fell back everywhere. Interfaces now carry an inline function's
/// body as rendered Move source, so the dependent expands it exactly as it
/// would from a source dependency.
///
/// The assertion that matters is the one on `Mid`: it exports an inline
/// function and must still *publish* an interface. If it stopped, every
/// dependent would quietly return to compiling sources and only the timings
/// would show it.
#[test]
fn a_cross_package_inline_call_is_served_by_the_interface() {
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
        "serving the inline call from the interface changed the produced bytecode"
    );

    // Every package publishes an interface, including the one exporting the
    // inline function — that is what keeps `Root` on the modular path.
    let build_root = dir
        .path()
        .join("modular")
        .join(CompiledPackageLayout::Root.path());
    for package in ["Leaf", "Mid", "Root"] {
        let interfaces = build_root
            .join(package)
            .join(CompiledPackageLayout::CompiledInterfaces.path());
        assert!(
            interfaces.is_dir() && fs::read_dir(&interfaces).unwrap().count() > 0,
            "`{package}` published no interface, so its dependents fell back to sources"
        );
    }

    // And the inline function crossed as source, not as a `native` stub.
    let mid = fs::read_to_string(
        build_root
            .join("Mid")
            .join(CompiledPackageLayout::CompiledInterfaces.path())
            .join("0xcafe_mid.xir.json"),
    )
    .unwrap();
    assert!(
        mid.contains("inline fun bump"),
        "`Mid`'s interface does not carry the inline body"
    );
}

/// A prebuilt dependency cache is read, never written, and changes nothing
/// about the result.
///
/// This is the arrangement that lets many builds share one set of dependencies
/// without sharing a *writable* directory. `e2e-move-tests` relies on it: the
/// framework is built once into a shared directory, every test reads it from
/// there and writes its own output elsewhere, and only because no two builds
/// write the same place can they skip the machine-global package lock.
///
/// Two claims, and the second is the one that makes it safe. The dependencies
/// must be *reused* — otherwise the cache is pointless — and the cache must come
/// back untouched, since a concurrent reader would otherwise see files being
/// rewritten underneath it.
#[test]
fn a_dependency_cache_is_reused_without_being_written() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);

    // An ordinary modular build fills the cache.
    let cache = dir.path().join("cache");
    let expected = compile(&chain, &cache, true);
    let before = directory_state(&cache);
    assert!(!before.is_empty(), "the cache was not populated");

    // Build again, reading that cache but writing somewhere else entirely.
    let install = dir.path().join("install");
    let mut log = Vec::new();
    let package = BuildConfig {
        install_dir: Some(install.clone()),
        dependency_cache_dir: Some(cache.clone()),
        modular_compilation: true,
        ..Default::default()
    }
    .compile_package(&chain.join("root"), &mut log)
    .expect("building against a dependency cache");
    let log = String::from_utf8_lossy(&log).into_owned();

    assert_eq!(
        cached_packages(&log),
        vec!["Leaf", "Mid"],
        "dependencies must come from the cache, not be rebuilt; log was:\n{log}"
    );
    assert_eq!(
        directory_state(&cache),
        before,
        "the dependency cache must be read-only"
    );

    let actual = package
        .root_compiled_units
        .iter()
        .chain(package.deps_compiled_units.iter().map(|(_, unit)| unit))
        .map(|unit| (unit.unit.name().to_string(), unit.unit.serialize(None)))
        .collect::<BTreeMap<_, _>>();
    for (name, bytes) in &expected {
        assert_eq!(
            bytes,
            actual
                .get(name)
                .unwrap_or_else(|| panic!("`{name}` is missing")),
            "`{name}` differs when built against a dependency cache"
        );
    }
}

/// Every file under `dir`, with its size and modification time.
///
/// Compared before and after to show a directory was not written. Size alone
/// would miss a rewrite with identical content, which still breaks a concurrent
/// reader because `save_to_disk` clears the directory first.
fn directory_state(dir: &Path) -> BTreeMap<PathBuf, (u64, std::time::SystemTime)> {
    fn walk(dir: &Path, into: &mut BTreeMap<PathBuf, (u64, std::time::SystemTime)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.metadata() {
                Ok(meta) if meta.is_dir() => walk(&path, into),
                Ok(meta) => {
                    let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                    into.insert(path, (meta.len(), modified));
                },
                Err(_) => {},
            }
        }
    }
    let mut state = BTreeMap::new();
    walk(dir, &mut state);
    state
}

/// An edited interface in the build directory is not served from cache.
///
/// Everything else about cache validity is decided from `BuildInfo.yaml`, but
/// the *interface files* beside it are what the compiler is handed, and they
/// carry inline function bodies as source that a dependent expands into its own
/// bytecode. `build/` is git-ignored and freely writable, so trusting the
/// metadata without the bytes it describes would let an edited cache change
/// what a build produces from unchanged sources.
#[test]
fn an_edited_cached_interface_is_not_reused() {
    let dir = tempfile::tempdir().unwrap();
    let chain = chain_in(&dir, MID, ROOT);
    let install = dir.path().join("modular");
    compile_logged(&chain, &install, true);

    // Unchanged, everything is reused — the baseline the next assertion needs.
    let (_, log) = compile_logged(&chain, &install, true);
    assert_eq!(
        cached_packages(&log),
        vec!["Leaf", "Mid", "Root"],
        "nothing changed, so everything should have been reused; log was:\n{log}"
    );

    // Now edit one of Leaf's published interfaces in place, leaving its
    // `BuildInfo.yaml` and its sources untouched.
    let interfaces = install
        .join(CompiledPackageLayout::Root.path())
        .join("Leaf")
        .join(CompiledPackageLayout::CompiledInterfaces.path());
    let target = fs::read_dir(&interfaces)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "json"))
        .expect("Leaf published an interface");
    let edited = fs::read_to_string(&target)
        .unwrap()
        .replace("\"read\"", "\"read_tampered\"");
    fs::write(&target, edited).unwrap();

    let (_, log) = compile_logged(&chain, &install, true);
    assert!(
        !cached_packages(&log).contains(&"Leaf".to_string()),
        "an edited interface must not be served from cache; log was:\n{log}"
    );
}
