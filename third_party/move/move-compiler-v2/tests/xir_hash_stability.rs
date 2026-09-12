// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Stability of the interface hash, measured on real compilation.
//!
//! The unit tests in `xir_hash` mutate a hand-written interface and check the
//! digest responds. That establishes the projection is right; it cannot
//! establish that two *builds* agree, because the things most likely to leak in
//! — symbol interning order, table positions, absolute paths — are properties
//! of a build, not of a document.
//!
//! Both directions matter and they fail differently. A hash that moves when it
//! should not merely wastes work; a hash that stays when it should not produces
//! a stale build, which is the failure this digest exists to prevent.

use move_compiler_v2::{run_checker, xir_export, xir_hash, Options};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use std::{collections::BTreeMap, fs, path::Path};

/// Compiles `sources` (name → text) in a fresh directory under `prefix` and
/// returns each target module's interface hash, keyed by module name.
fn hashes_of(prefix: &str, sources: &BTreeMap<&str, &str>) -> BTreeMap<String, String> {
    let dir = tempfile::Builder::new().prefix(prefix).tempdir().unwrap();
    hashes_in(dir.path(), sources)
}

fn hashes_in(dir: &Path, sources: &BTreeMap<&str, &str>) -> BTreeMap<String, String> {
    let paths = sources
        .iter()
        .map(|(name, text)| {
            let path = dir.join(name);
            fs::write(&path, text).unwrap();
            path.to_string_lossy().into_owned()
        })
        .collect::<Vec<_>>();

    let env = run_checker(Options {
        sources: paths,
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        skip_attribute_checks: true,
        ..Options::default()
    })
    .expect("compiling");
    let mut diagnostics = codespan_reporting::term::termcolor::Buffer::no_color();
    env.report_diag(
        &mut diagnostics,
        codespan_reporting::diagnostic::Severity::Warning,
    );
    assert!(
        !env.has_errors(),
        "compiling failed:\n{}",
        String::from_utf8_lossy(&diagnostics.into_inner())
    );

    env.get_modules()
        .filter(|module| module.is_primary_target())
        .map(|module| {
            let interface = xir_export::export_interface(&module)
                .unwrap_or_else(|e| panic!("exporting `{}`: {:#}", module.get_full_name_str(), e));
            (
                module.get_full_name_str(),
                xir_hash::interface_hash(&interface).unwrap(),
            )
        })
        .collect()
}

const LIB: &str = r#"
module 0xcafe::lib {
    friend 0xcafe::other;

    struct Holder<T: store> has store, drop { item: T, tag: u8 }
    public enum Shape has copy, drop { Point, Line(u64) }

    public fun hold<T: store>(item: T, tag: u8): Holder<T> { Holder { item, tag } }
    public fun area(s: &Shape): u64 {
        match (s) { Shape::Point => 0, Shape::Line(n) => *n }
    }
    /// The signature can be varied without touching the body, so a test can
    /// change one element of the API at a time.
    public fun scale(n: u64): u64 { 0 }
    public(friend) fun internal(): u64 { 1 }
    fun hidden(): u64 { 2 }
}
"#;

/// Exactly [`LIB`]'s declarations, in a different order. Written out rather
/// than derived by string surgery so that a rewrite bug cannot masquerade as a
/// hash bug — the first version of this test silently deleted a function and
/// looked like a canonicalization failure.
const LIB_REORDERED: &str = r#"
module 0xcafe::lib {
    friend 0xcafe::other;

    public enum Shape has copy, drop { Point, Line(u64) }
    struct Holder<T: store> has store, drop { item: T, tag: u8 }

    fun hidden(): u64 { 2 }
    public fun scale(n: u64): u64 { 0 }
    public(friend) fun internal(): u64 { 1 }
    public fun area(s: &Shape): u64 {
        match (s) { Shape::Point => 0, Shape::Line(n) => *n }
    }
    public fun hold<T: store>(item: T, tag: u8): Holder<T> { Holder { item, tag } }
}
"#;

const OTHER: &str = r#"
module 0xcafe::other {
    public fun peek(): u64 { 0xcafe::lib::internal() }
}
"#;

fn baseline() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([("lib.move", LIB), ("other.move", OTHER)])
}

/// Two builds of identical source agree, including from different directories.
///
/// The directory matters: hashing `.mv` needs a canonicalization pass precisely
/// because independent recompilations differ, and `MODULAR_COMPILATION.md` §3
/// offers "no canonicalization needed" as a reason to prefer XIR. That claim is
/// worth testing rather than assuming — and it is only true because the hash
/// resolves table indices to names and sorts declarations, not because the
/// exported document itself is canonical.
#[test]
fn identical_sources_hash_identically_from_different_paths() {
    let first = hashes_of("xir-hash-a", &baseline());
    let second = hashes_of("xir-hash-b-longer-prefix", &baseline());
    assert!(!first.is_empty(), "no modules were hashed");
    assert_eq!(first, second, "the build directory leaked into the hash");
}

/// Reordering the source files must not matter either.
///
/// Symbol ids are assigned in interning order, so the order files reach the
/// parser is exactly what determines declaration order in the export. Feeding
/// the same modules in a different order is the cheapest way to shake that out.
#[test]
fn source_file_order_does_not_affect_the_hash() {
    // `BTreeMap` iterates by key, so change the keys to change the order while
    // keeping the content identical.
    let forward = BTreeMap::from([("a_lib.move", LIB), ("b_other.move", OTHER)]);
    let reverse = BTreeMap::from([("a_other.move", OTHER), ("b_lib.move", LIB)]);
    let first = hashes_of("xir-hash-fwd", &forward);
    let second = hashes_of("xir-hash-rev", &reverse);
    assert_eq!(first, second, "source file order leaked into the hash");
}

/// Source order does not reach the export at all — an invariant, not a
/// coincidence, and worth pinning because the hash's canonicalization was
/// designed on the opposite assumption.
///
/// The reasoning that motivated sorting in `xir_hash` was: a declaration's
/// position comes from `BTreeMap<StructId, _>`, `StructId` wraps
/// `Symbol(usize)`, and a symbol's number is assigned when first interned — so
/// order should follow the source. Measurement says otherwise: reordering every
/// declaration in a module produces a **byte-identical** export.
///
/// The reason is an incidental alignment of two unrelated maps. Expansion holds
/// module members in `UniqueMap`, which is a `BTreeMap` keyed by the name
/// *string* (`unique_map.rs:15`), so the model builder visits them in name
/// order and interns them in name order; the model's index-keyed `BTreeMap`
/// then agrees with the string-keyed one.
///
/// Two consequences. The canonicalization is not currently load-bearing for
/// declaration order — but it costs nothing and does not depend on that
/// alignment holding, which is the right way round for something a cache's
/// correctness rests on. And if this test ever fails, the alignment has broken
/// and the sorting in `xir_hash` has quietly become essential.
#[test]
fn source_declaration_order_does_not_reach_the_export() {
    assert_eq!(
        raw_export("xir-hash-order-a", &baseline()),
        raw_export(
            "xir-hash-order-b",
            &BTreeMap::from([("lib.move", LIB_REORDERED), ("other.move", OTHER)])
        ),
        "declaration order now reaches the export; the hash's sorting is load-bearing"
    );
}

/// The exported interface of `0xcafe::lib`, as JSON, before canonicalization.
fn raw_export(prefix: &str, sources: &BTreeMap<&str, &str>) -> String {
    let dir = tempfile::Builder::new().prefix(prefix).tempdir().unwrap();
    let paths = sources
        .iter()
        .map(|(name, text)| {
            let path = dir.path().join(name);
            fs::write(&path, text).unwrap();
            path.to_string_lossy().into_owned()
        })
        .collect::<Vec<_>>();
    let env = run_checker(Options {
        sources: paths,
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        skip_attribute_checks: true,
        ..Options::default()
    })
    .unwrap();
    assert!(!env.has_errors());
    let module = env
        .get_modules()
        .find(|module| module.get_full_name_str() == "0xcafe::lib")
        .expect("lib was compiled");
    serde_json::to_string(&xir_export::export_interface(&module).unwrap()).unwrap()
}

/// Editing a body — or a private function — leaves dependents alone.
///
/// If these moved the hash, every implementation change would rebuild the
/// world and the cache would rarely hit.
#[test]
fn implementation_changes_do_not_move_the_hash() {
    let baseline_hashes = hashes_of("xir-hash-base", &baseline());

    for (label, lib) in [
        (
            "a public function's body",
            LIB.replace("Shape::Point => 0", "Shape::Point => 1"),
        ),
        (
            "a private function's body",
            LIB.replace("fun hidden(): u64 { 2 }", "fun hidden(): u64 { 3 }"),
        ),
        (
            "a private function added",
            LIB.replace(
                "fun hidden(): u64 { 2 }",
                "fun hidden(): u64 { 2 }\n    fun extra(): u64 { 9 }",
            ),
        ),
        (
            "a comment",
            LIB.replace("module 0xcafe::lib {", "module 0xcafe::lib { // note"),
        ),
    ] {
        let edited = hashes_of(
            "xir-hash-edit",
            &BTreeMap::from([("lib.move", lib.as_str()), ("other.move", OTHER)]),
        );
        assert_eq!(
            baseline_hashes, edited,
            "changing {label} moved the interface hash"
        );
    }
}

/// Editing the API does move it.
///
/// The complement of the test above: without this, a hash that never changed
/// would pass it trivially.
#[test]
fn api_changes_move_the_hash() {
    let baseline_hashes = hashes_of("xir-hash-base2", &baseline());

    for (label, lib) in [
        (
            "a parameter type",
            LIB.replace("scale(n: u64)", "scale(n: u128)"),
        ),
        (
            "a return type",
            LIB.replace("scale(n: u64): u64", "scale(n: u64): u128"),
        ),
        (
            "an ability",
            LIB.replace(
                "enum Shape has copy, drop",
                "enum Shape has copy, drop, store",
            ),
        ),
        (
            "a field name",
            LIB.replace("{ item: T, tag: u8 }", "{ item: T, label: u8 }")
                .replace("Holder { item, tag }", "Holder { item, label: tag }"),
        ),
        (
            "an enum variant",
            LIB.replace("Point, Line(u64)", "Point, Line(u64), Rect"),
        ),
        (
            "a visibility",
            LIB.replace("public(friend) fun internal", "public fun internal"),
        ),
        (
            "a friend declaration",
            LIB.replace(
                "friend 0xcafe::other;",
                "friend 0xcafe::other;\n    friend 0xcafe::third;",
            ),
        ),
        (
            "a new public function",
            LIB.replace(
                "fun hidden(): u64 { 2 }",
                "fun hidden(): u64 { 2 }\n    public fun added(): u64 { 4 }",
            ),
        ),
    ] {
        let mut sources = BTreeMap::from([("lib.move", lib.as_str()), ("other.move", OTHER)]);
        // A new friend target must exist for the module to be modelled at all.
        if label == "a friend declaration" {
            sources.insert(
                "third.move",
                "module 0xcafe::third { public fun t(): u64 { 0 } }",
            );
        }
        let edited = hashes_of("xir-hash-api", &sources);
        assert_ne!(
            baseline_hashes.get("0xcafe::lib"),
            edited.get("0xcafe::lib"),
            "changing {label} left the interface hash unchanged"
        );
    }
}
