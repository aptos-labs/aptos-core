// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos extended checks must reach the same verdict whether a dependency is
//! supplied as source or as an XIR interface.
//!
//! The randomness check is the sharpest case. `check_unsafe_randomness_usage`
//! walks `get_called_functions()` transitively to reject a public function that
//! can reach `0x1::randomness`. An interface declares every function `native`,
//! so it carries no callees — and a traversal that stops at a dependency
//! reports "safe" for code the monolithic build rejects. That is a safety
//! check silently weakening, which is the one way this path could be worse
//! than wrong.

use aptos_framework::extended_checks::run_extended_checks;
use move_compiler_v2::{run_checker, run_move_compiler_to_model, xir_export, Options};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use std::{fs, path::Path};

/// A stand-in for `0x1::randomness`; the check keys on the module path, not on
/// the real framework, so this is enough to exercise it without building it.
const RANDOMNESS: &str = r#"
module 0x1::randomness {
    public fun u64_integer(): u64 { 1 }
}
"#;

/// The indirection under test: a dependency function that is neither `inline`
/// nor itself a randomness function, but reaches one.
const DEPENDENCY: &str = r#"
module 0xcafe::dep {
    public fun helper(): u64 { 0x1::randomness::u64_integer() }
}
"#;

/// A `public` function reaching randomness through that dependency. The
/// monolithic build rejects this.
///
/// `keep_loaded` is private, so it is not itself checked; it stands in for the
/// framework dependency a real package always has. Without something naming
/// the module, replacing `dep`'s body with an interface prunes it from the
/// build entirely and there is nothing left to detect.
const TARGET: &str = r#"
module 0xcafe::target {
    public fun exposed(): u64 { 0xcafe::dep::helper() }
    fun keep_loaded(): u64 { 0x1::randomness::u64_integer() }
}
"#;

/// The same target, but with nothing naming `0x1::randomness` directly.
///
/// This is the degraded case: with `dep`'s body replaced by an interface,
/// nothing else references the randomness module, so it is pruned from the
/// build and the recorded call edge cannot resolve.
const TARGET_WITHOUT_DIRECT_REFERENCE: &str = r#"
module 0xcafe::target {
    public fun exposed(): u64 { 0xcafe::dep::helper() }
}
"#;

fn options(sources: Vec<String>, dependencies: Vec<String>, xir: Vec<String>) -> Options {
    Options {
        sources,
        dependencies,
        xir_dependencies: xir,
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        skip_attribute_checks: true,
        ..Options::default()
    }
}

/// Runs extended checks over `target` and reports whether they complained.
///
/// Compiled through `run_move_compiler_to_model` rather than `run_checker`,
/// because that is what a real build does before running extended checks
/// (`built_package.rs`) — it attaches the compiled modules the model needs.
fn extended_checks_reject(options: Options) -> bool {
    let env = run_move_compiler_to_model(options).expect("the build ran");
    assert!(!env.has_errors(), "the build itself failed");
    run_extended_checks(&env);
    env.has_errors()
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

/// A build missing a module an interface calls into must say so.
///
/// With `dep`'s body replaced by an interface, nothing else names
/// `0x1::randomness`, so it is pruned. Dropping the edge would make the safety
/// check pass on code the source build rejects, so the build reports the
/// incomplete closure instead — the package system supplies it through
/// transitive dependencies.
#[test]
fn a_pruned_callee_module_fails_the_build_rather_than_the_check() {
    let dir = tempfile::Builder::new()
        .prefix("xir-randomness-pruned")
        .tempdir()
        .unwrap();
    let randomness = write(dir.path(), "randomness.move", RANDOMNESS);
    let dependency = write(dir.path(), "dep.move", DEPENDENCY);
    let target = write(dir.path(), "target.move", TARGET_WITHOUT_DIRECT_REFERENCE);

    // Source dependencies: the walk reaches randomness and rejects.
    assert!(
        extended_checks_reject(options(
            vec![target.clone()],
            vec![randomness.clone(), dependency.clone()],
            vec![],
        )),
        "the monolithic build must still reject"
    );

    let env = run_checker(options(vec![dependency], vec![randomness.clone()], vec![]))
        .expect("modelling the dependency");
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the dependency was modelled");
    let interface = xir_export::export_interface(&module).expect("the dependency exports");
    let interface_path = dir.path().join("dep.xir.json");
    fs::write(&interface_path, serde_json::to_string(&interface).unwrap()).unwrap();

    let error = run_move_compiler_to_model(options(vec![target], vec![randomness], vec![
        interface_path.to_string_lossy().into_owned(),
    ]))
    .err()
    .map(|error| format!("{error:#}"))
    .unwrap_or_else(|| panic!("the build accepted an interface whose callee module is absent"));
    assert!(
        error.contains("0x1::randomness") && error.contains("not in the build"),
        "the error must name the missing module: {error}"
    );
}

#[test]
fn transitive_randomness_is_rejected_through_an_interface_as_it_is_through_source() {
    let dir = tempfile::Builder::new()
        .prefix("xir-randomness")
        .tempdir()
        .unwrap();
    let randomness = write(dir.path(), "randomness.move", RANDOMNESS);
    let dependency = write(dir.path(), "dep.move", DEPENDENCY);
    let target = write(dir.path(), "target.move", TARGET);

    // The reference: the dependency's body is visible, so the walk reaches
    // `0x1::randomness` and the public function is rejected.
    assert!(
        extended_checks_reject(options(
            vec![target.clone()],
            vec![randomness.clone(), dependency.clone()],
            vec![],
        )),
        "the monolithic build must reject a public function that reaches randomness"
    );

    // The same program, with the dependency described by its interface.
    let env = run_checker(options(vec![dependency], vec![randomness.clone()], vec![]))
        .expect("modelling the dependency");
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the dependency was modelled");
    let interface = xir_export::export_interface(&module).expect("the dependency exports");
    let interface_path = dir.path().join("dep.xir.json");
    fs::write(&interface_path, serde_json::to_string(&interface).unwrap()).unwrap();

    assert!(
        extended_checks_reject(options(vec![target], vec![randomness], vec![
            interface_path.to_string_lossy().into_owned()
        ],)),
        "an interface-supplied dependency must not hide a transitive randomness \
         call from the safety check — the check walks `get_called_functions()`, \
         and an interface declares every function `native`"
    );
}
