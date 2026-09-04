// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Sweeps the three framework packages through the XIR interface exporter.
//!
//! This is the coverage measurement for XIR's dependency-description path:
//! modular compilation only works if every module a target can depend on has
//! an exportable interface. The framework is the largest body of Move that
//! ships with the repo, so "every framework module exports" is the strongest
//! available evidence that the exporter's coverage is real rather than fitted
//! to hand-written tests.
//!
//! The sweep is end-to-end: it exports, serializes to JSON, and reads the
//! whole package back with the real reader. A form the exporter emits but the
//! reader cannot consume therefore fails here, rather than at dependency
//! resolution time.

use aptos_framework::{build_model, path_in_crate};
use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_compiler_v2::{
    run_checker,
    xir::{import_sources, parse_interface},
    xir_export::export_interface,
    Options,
};
use move_model::{
    metadata::{CompilerVersion, LanguageVersion},
    model::GlobalEnv,
};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

fn model_for(package: &str) -> GlobalEnv {
    let env = build_model(
        /*dev_mode*/ false,
        /*test_mode*/ false,
        /*verify_mode*/ false,
        &path_in_crate(package),
        BTreeMap::new(),
        /*target_filter*/ None,
        /*bytecode_version*/ None,
        Some(CompilerVersion::latest_stable()),
        Some(LanguageVersion::latest_stable()),
        /*skip_attribute_checks*/ true,
        BTreeSet::new(),
        /*experiments*/ vec![],
        /*with_bytecode*/ false,
        /*all_files_as_targets*/ false,
    )
    .unwrap_or_else(|e| panic!("building `{}` failed: {}", package, e));
    env.check_errors(&format!("compiling `{}`", package))
        .unwrap_or_else(|e| panic!("{}", e));
    env
}

/// Exports every module of `package` — including the dependency modules it
/// pulls in, since those are exactly what a modular build would describe —
/// and reads the resulting set back. Failures accumulate so that one run
/// reports the whole surface instead of stopping at the first gap.
fn sweep(package: &str) {
    let env = model_for(package);
    let mut failures = vec![];
    let mut sources = vec![];
    for module in env.get_modules() {
        let name = module.get_full_name_str();
        let interface = match export_interface(&module) {
            Ok(interface) => interface,
            Err(e) => {
                // `{:#}` so the whole `with_context` chain is reported; the
                // outermost layer only names the declaration, not the cause.
                failures.push(format!("{}: export failed: {:#}", name, e));
                continue;
            },
        };
        let json = match serde_json::to_string(&interface) {
            Ok(json) => json,
            Err(e) => {
                failures.push(format!("{}: interface does not serialize: {}", name, e));
                continue;
            },
        };
        match parse_interface(
            PathBuf::from(format!("{}.xir.json", name)),
            String::new(),
            &json,
        ) {
            Ok(source) => sources.push(source),
            Err(e) => failures.push(format!("{}: interface does not parse back: {}", name, e)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} modules failed:\n{}",
        failures.len(),
        sources.len() + failures.len(),
        failures.join("\n")
    );

    // Load the whole package as one dependency set: this resolves every
    // cross-module reference the exporter emitted, so a mis-encoded external
    // type or function shows up as an unresolved import.
    let exported = sources.len();
    let mut round_trip = GlobalEnv::new();
    let mut targets = FunctionTargetsHolder::default();
    import_sources(&mut round_trip, &sources, &mut targets)
        .unwrap_or_else(|e| panic!("`{}` interfaces do not load: {}", package, e));
    assert_eq!(
        round_trip.get_module_count(),
        exported,
        "`{}`: not every exported interface came back",
        package
    );
    println!("{}: {} modules exported and reloaded", package, exported);
}

/// Lowers every exported interface of `package` to Move source and type-checks
/// the whole set as compiler dependencies.
///
/// This is the other half of the round trip: `sweep` proves the XIR reader
/// accepts what the exporter emits, and this proves the *Move front end* does
/// too, once the interface is lowered to source. The interfaces reference each
/// other by fully-qualified path, so checking them together also exercises
/// cross-module resolution.
fn check_generated_interfaces(package: &str) {
    let env = model_for(package);
    let dir = tempfile::Builder::new()
        .prefix("xir-interface-sweep")
        .tempdir()
        .unwrap();
    let mut xir_dependencies = vec![];
    for module in env.get_modules() {
        let interface = export_interface(&module).unwrap_or_else(|e| {
            panic!("exporting `{}` failed: {:#}", module.get_full_name_str(), e)
        });
        let path = dir.path().join(format!(
            "{}.xir.json",
            module.get_full_name_str().replace("::", "_")
        ));
        std::fs::write(&path, serde_json::to_string(&interface).unwrap()).unwrap();
        xir_dependencies.push(path.to_string_lossy().into_owned());
    }

    // A target is required, but its content is irrelevant: what is under test
    // is whether the generated dependencies parse and type-check.
    let target = dir.path().join("target.move");
    std::fs::write(&target, "module 0xf00d::xir_interface_target {}\n").unwrap();

    let count = xir_dependencies.len();
    let checked = run_checker(Options {
        sources: vec![target.to_string_lossy().into_owned()],
        xir_dependencies,
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        // The framework's attributes (`resource_group`, `randomness`, …) are
        // Aptos-specific and unknown to the compiler's built-in set; a real
        // build passes them in. Attribute *checking* is not what is under test.
        skip_attribute_checks: true,
        ..Options::default()
    })
    .unwrap_or_else(|e| panic!("`{}` generated interfaces failed to build: {}", package, e));

    let mut diagnostics = Buffer::no_color();
    checked.report_diag(&mut diagnostics, Severity::Warning);
    assert!(
        !checked.has_errors(),
        "`{}` generated interfaces do not type-check:\n{}",
        package,
        String::from_utf8_lossy(&diagnostics.into_inner())
    );
    println!("{}: {} generated interfaces type-check", package, count);
}

#[test]
fn move_stdlib_interfaces_export() {
    sweep("move-stdlib");
}

#[test]
fn move_stdlib_generated_interfaces_check() {
    check_generated_interfaces("move-stdlib");
}

#[test]
fn aptos_stdlib_generated_interfaces_check() {
    check_generated_interfaces("aptos-stdlib");
}

#[test]
fn aptos_framework_generated_interfaces_check() {
    check_generated_interfaces("aptos-framework");
}

#[test]
fn aptos_stdlib_interfaces_export() {
    sweep("aptos-stdlib");
}

#[test]
fn aptos_framework_interfaces_export() {
    sweep("aptos-framework");
}
