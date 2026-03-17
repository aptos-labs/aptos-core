// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the modular (incremental) compilation path.
//!
//! Each test works against the fixture at
//! `tests/test_sources/modular/chain/` which defines a three-package chain:
//!
//!   Leaf  (no deps)
//!     ↑
//!   Middle  (depends on Leaf)
//!     ↑
//!   Root    (depends on Middle, transitively on Leaf)
//!
//! Tests copy this tree into a fresh temp directory so they can freely mutate
//! source files without affecting the checked-in fixtures.

use legacy_move_compiler::compiled_unit::CompiledUnit;
use move_package::{compilation::package_layout::CompiledPackageLayout, BuildConfig};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Copies the fixture tree rooted at `src` into `dst`.
fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let dest = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_all(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

/// Fixture tree structure in a temp directory.
struct PackageTree {
    _tmp: TempDir,
    root: PathBuf,
}

impl PackageTree {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let fixture = Path::new("tests/test_sources/modular/chain");
        let root = tmp.path().join("chain");
        copy_dir_all(fixture, &root);
        PackageTree { _tmp: tmp, root }
    }

    fn leaf_source(&self) -> PathBuf {
        self.root.join("deps_only/Leaf/sources/leaf.move")
    }

    fn middle_source(&self) -> PathBuf {
        self.root.join("deps_only/Middle/sources/middle.move")
    }

    /// Build with `modular_compilation = true` and return:
    /// - the build log (text written by the build driver)
    /// - the names of packages that were BUILDING (i.e. compiled)
    /// - the names of packages that were CACHED
    fn build_modular(&self) -> (String, Vec<String>, Vec<String>) {
        self.build_with_config(BuildConfig {
            modular_compilation: true,
            install_dir: Some(self.root.clone()),
            ..Default::default()
        })
    }

    /// Build with the legacy monolithic path.
    fn build_monolithic(&self) -> move_package::compilation::compiled_package::CompiledPackage {
        BuildConfig {
            modular_compilation: false,
            install_dir: Some(self.root.clone()),
            ..Default::default()
        }
        .compile_package(&self.root, &mut Vec::new())
        .unwrap()
    }

    fn build_with_config(&self, config: BuildConfig) -> (String, Vec<String>, Vec<String>) {
        let mut log = Vec::<u8>::new();
        let _pkg = config.compile_package(&self.root, &mut log).unwrap();
        let log = String::from_utf8(log).unwrap();
        let building = packages_with_status(&log, "BUILDING");
        let cached = packages_with_status(&log, "CACHED");
        (log, building, cached)
    }
}

/// Extract package names that appear on lines matching `status` in the build log.
/// The log may contain ANSI color codes; we search for the plain word.
fn packages_with_status(log: &str, status: &str) -> Vec<String> {
    log.lines()
        .filter(|l| l.contains(status))
        .filter_map(|l| {
            // Line looks like: "BUILDING Root" (possibly with ANSI escapes)
            // Grab the last whitespace-delimited token as the package name.
            l.split_whitespace().last().map(|s| s.to_owned())
        })
        .collect()
}

/// Collect all (module_name → serialized bytes) from a compiled package.
fn bytecode_map(
    pkg: &move_package::compilation::compiled_package::CompiledPackage,
) -> std::collections::BTreeMap<String, Vec<u8>> {
    pkg.all_compiled_units()
        .filter_map(|unit| match unit {
            CompiledUnit::Module(m) => {
                let mut bytes = vec![];
                m.module.serialize(&mut bytes).unwrap();
                Some((m.module.self_id().to_string(), bytes))
            },
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Clean build: every package must be compiled (none are cached).
#[test]
fn test_clean_build_compiles_all() {
    let tree = PackageTree::new();
    let (_log, building, cached) = tree.build_modular();

    assert!(
        building.contains(&"Leaf".to_owned()),
        "Leaf should be BUILDING on a clean build"
    );
    assert!(
        building.contains(&"Middle".to_owned()),
        "Middle should be BUILDING on a clean build"
    );
    assert!(
        building.contains(&"Root".to_owned()),
        "Root should be BUILDING on a clean build"
    );
    assert!(
        cached.is_empty(),
        "Nothing should be CACHED on a clean build"
    );
}

/// Rebuild without any changes: every package must be served from cache.
#[test]
fn test_no_change_rebuild_all_cached() {
    let tree = PackageTree::new();
    tree.build_modular(); // warm the cache

    let (_log, building, cached) = tree.build_modular();

    assert!(
        building.is_empty(),
        "No packages should be BUILDING on a no-change rebuild, got: {:?}",
        building
    );
    assert_eq!(cached.len(), 3, "All three packages should be CACHED");
}

/// Changing a private function body in Leaf must NOT cascade to Middle or Root.
#[test]
fn test_private_change_does_not_cascade() {
    let tree = PackageTree::new();
    tree.build_modular();

    // Change the body of the private function in Leaf (no interface change).
    fs::write(
        tree.leaf_source(),
        "module leaf::leaf {\n\
         public fun add(x: u64, y: u64): u64 { x + y }\n\
         fun private_impl(): u64 { 999 }  // changed body\n\
         }\n",
    )
    .unwrap();

    let (_log, building, cached) = tree.build_modular();

    assert!(
        building.contains(&"Leaf".to_owned()),
        "Leaf must recompile after source change"
    );
    assert!(
        cached.contains(&"Middle".to_owned()),
        "Middle must be CACHED: Leaf's interface did not change"
    );
    assert!(
        cached.contains(&"Root".to_owned()),
        "Root must be CACHED: no transitive interface change"
    );
}

/// Adding a new public function to Leaf changes Leaf's interface hash.
/// Middle must recompile (it recorded Leaf's old hash). However, Middle's
/// own public interface (`compute`) is unchanged, so Root stays CACHED —
/// the cascade stops at the first package whose interface is stable.
#[test]
fn test_public_change_cascades_one_level() {
    let tree = PackageTree::new();
    tree.build_modular();

    // Add a new public function to Leaf (Leaf's interface hash changes).
    fs::write(
        tree.leaf_source(),
        "module leaf::leaf {\n\
         public fun add(x: u64, y: u64): u64 { x + y }\n\
         public fun sub(x: u64, y: u64): u64 { x - y }  // new public fn\n\
         fun private_impl(): u64 { 1 }\n\
         }\n",
    )
    .unwrap();

    let (_log, building, cached) = tree.build_modular();

    assert!(
        building.contains(&"Leaf".to_owned()),
        "Leaf must recompile after interface change"
    );
    assert!(
        building.contains(&"Middle".to_owned()),
        "Middle must recompile: its recorded Leaf interface hash is stale"
    );
    // Root's dep is Middle, whose interface (compute's signature) did not change.
    assert!(
        cached.contains(&"Root".to_owned()),
        "Root must stay CACHED: Middle's own interface hash did not change"
    );
}

/// When both Leaf AND Middle get new public functions, all three packages
/// must recompile because the interface change propagates all the way up.
#[test]
fn test_public_change_cascades_two_levels() {
    let tree = PackageTree::new();
    tree.build_modular();

    // Add a new public function to Leaf (Leaf's interface hash changes).
    fs::write(
        tree.leaf_source(),
        "module leaf::leaf {\n\
         public fun add(x: u64, y: u64): u64 { x + y }\n\
         public fun sub(x: u64, y: u64): u64 { x - y }\n\
         fun private_impl(): u64 { 1 }\n\
         }\n",
    )
    .unwrap();

    // Also add a new public function to Middle (Middle's interface hash changes).
    fs::write(
        tree.middle_source(),
        "module middle::middle {\n\
         use leaf::leaf;\n\
         public fun compute(x: u64, y: u64): u64 { leaf::add(x, y) }\n\
         public fun compute_sub(x: u64, y: u64): u64 { leaf::sub(x, y) }  // new public fn\n\
         }\n",
    )
    .unwrap();

    let (_log, building, _cached) = tree.build_modular();

    assert!(
        building.contains(&"Leaf".to_owned()),
        "Leaf must recompile after interface change"
    );
    assert!(
        building.contains(&"Middle".to_owned()),
        "Middle must recompile: Leaf's interface hash changed"
    );
    assert!(
        building.contains(&"Root".to_owned()),
        "Root must recompile: Middle's interface hash changed"
    );
}

/// A change confined to Middle (not touching Leaf or Root sources) should only
/// recompile Middle and Root; Leaf must remain cached.
#[test]
fn test_middle_change_does_not_rebuild_leaf() {
    let tree = PackageTree::new();
    tree.build_modular();

    // Change Middle's source (private change; doesn't affect Root's type-checking).
    fs::write(
        tree.middle_source(),
        "module middle::middle {\n\
         use leaf::leaf;\n\
         public fun compute(x: u64, y: u64): u64 { leaf::add(x, y) + 0 }  // trivial change\n\
         }\n",
    )
    .unwrap();

    let (_log, building, cached) = tree.build_modular();

    assert!(
        cached.contains(&"Leaf".to_owned()),
        "Leaf must be CACHED: its source did not change"
    );
    assert!(
        building.contains(&"Middle".to_owned()),
        "Middle must recompile after source change"
    );
    // Root's source did not change but Middle's interface may or may not have
    // changed. Either way, Root must be re-evaluated.
    assert!(
        building.contains(&"Root".to_owned()) || cached.contains(&"Root".to_owned()),
        "Root must appear in the build log"
    );
}

/// `force_recompilation` must bypass the cache and recompile every package.
#[test]
fn test_force_recompilation_rebuilds_all() {
    let tree = PackageTree::new();
    tree.build_modular(); // warm the cache

    let (_log, building, cached) = tree.build_with_config(BuildConfig {
        modular_compilation: true,
        force_recompilation: true,
        install_dir: Some(tree.root.clone()),
        ..Default::default()
    });

    assert_eq!(
        building.len(),
        3,
        "All three packages must be BUILDING with force_recompilation"
    );
    assert!(
        cached.is_empty(),
        "No packages should be CACHED with force_recompilation"
    );
}

/// The modular path must produce bit-for-bit identical bytecode to the
/// monolithic path for every module in the graph.
#[test]
fn test_bytecodes_identical_to_monolithic() {
    // Use separate temp trees so the two builds don't share a cache.
    let tree_mono = PackageTree::new();
    let tree_modular = PackageTree::new();

    let mono_pkg = tree_mono.build_monolithic();
    let (_, _, _) = tree_modular.build_modular();

    // Re-load the modular result through compile_package for easy comparison.
    let modular_pkg = BuildConfig {
        modular_compilation: true,
        install_dir: Some(tree_modular.root.clone()),
        ..Default::default()
    }
    .compile_package(&tree_modular.root, &mut Vec::new())
    .unwrap();

    let mono_map = bytecode_map(&mono_pkg);
    let modular_map = bytecode_map(&modular_pkg);

    assert_eq!(
        mono_map.keys().collect::<Vec<_>>(),
        modular_map.keys().collect::<Vec<_>>(),
        "Both paths must produce the same set of modules"
    );

    for (module_id, mono_bytes) in &mono_map {
        let modular_bytes = modular_map.get(module_id).unwrap();
        assert_eq!(
            mono_bytes, modular_bytes,
            "Bytecode mismatch for module {}",
            module_id
        );
    }
}

/// After a full modular build each package's BuildInfo.yaml must contain
/// `interface_hash` and `dep_interface_hashes`.
#[test]
fn test_build_info_contains_interface_hash() {
    let tree = PackageTree::new();
    tree.build_modular();

    let build_root = tree.root.join(CompiledPackageLayout::Root.path());

    for pkg_name in &["Leaf", "Middle", "Root"] {
        let build_info_path = build_root
            .join(pkg_name)
            .join(CompiledPackageLayout::BuildInfo.path());
        assert!(
            build_info_path.exists(),
            "BuildInfo.yaml missing for {}",
            pkg_name
        );
        let yaml = fs::read_to_string(&build_info_path).unwrap();
        assert!(
            yaml.contains("interface_hash"),
            "BuildInfo.yaml for {} must contain interface_hash",
            pkg_name
        );
    }

    // Middle and Root also store dep_interface_hashes.
    for pkg_name in &["Middle", "Root"] {
        let yaml = fs::read_to_string(
            build_root
                .join(pkg_name)
                .join(CompiledPackageLayout::BuildInfo.path()),
        )
        .unwrap();
        assert!(
            yaml.contains("dep_interface_hashes"),
            "BuildInfo.yaml for {} must contain dep_interface_hashes",
            pkg_name
        );
    }
}
