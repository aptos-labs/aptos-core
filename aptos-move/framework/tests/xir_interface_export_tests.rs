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
    xir_interface_generator::xir_module_to_move_source,
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

/// Exports every module of `env` — including the dependency modules the
/// package pulls in, since those are exactly what a modular build would
/// describe — and serializes each one, returning `(name, json)` pairs.
///
/// Both halves of the sweep consume the JSON rather than the `XirModule`, so
/// exporting and serializing happen once per package.
///
/// Failures accumulate so that one run reports the whole surface instead of
/// stopping at the first gap.
fn exported_interfaces(package: &str, env: &GlobalEnv) -> Vec<(String, String)> {
    let mut failures = vec![];
    let mut interfaces = vec![];
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
        match serde_json::to_string(&interface) {
            Ok(json) => interfaces.push((name, json)),
            Err(e) => failures.push(format!("{}: interface does not serialize: {}", name, e)),
        }
    }
    assert!(
        failures.is_empty(),
        "`{}`: {} of {} modules failed:\n{}",
        package,
        failures.len(),
        interfaces.len() + failures.len(),
        failures.join("\n")
    );
    interfaces
}

/// Reads every exported interface back with the real reader.
fn check_round_trip(package: &str, interfaces: &[(String, String)]) {
    let mut failures = vec![];
    let mut sources = vec![];
    for (name, json) in interfaces {
        match parse_interface(PathBuf::from(format!("{}.xir.json", name)), json) {
            Ok(source) => sources.push(source),
            Err(e) => failures.push(format!("{}: interface does not parse back: {}", name, e)),
        }
    }
    assert!(
        failures.is_empty(),
        "`{}`: {} of {} interfaces do not parse back:\n{}",
        package,
        failures.len(),
        interfaces.len(),
        failures.join("\n")
    );

    // Load the whole package as one dependency set: this resolves every
    // cross-module reference the exporter emitted, so a mis-encoded external
    // type or function shows up as an unresolved import.
    let mut round_trip = GlobalEnv::new();
    let mut targets = FunctionTargetsHolder::default();
    import_sources(&mut round_trip, &sources, &mut targets)
        .unwrap_or_else(|e| panic!("`{}` interfaces do not load: {:#}", package, e));
    assert_eq!(
        round_trip.get_module_count(),
        sources.len(),
        "`{}`: not every exported interface came back",
        package
    );
    println!(
        "{}: {} modules exported and reloaded",
        package,
        sources.len()
    );
}

/// Lowers every exported interface of `package` to Move source and type-checks
/// the whole set.
///
/// This is the other half of the round trip: `check_round_trip` proves the XIR
/// reader accepts what the exporter emits, and this proves the *Move front end*
/// does too, once the interface is lowered to source. The generated modules are
/// compiled as targets, so each is checked, inline bodies included. Passed as
/// dependencies they would be checked only when something reaches them.
fn check_generated_interfaces(package: &str, interfaces: &[(String, String)]) {
    let dir = tempfile::Builder::new()
        .prefix("xir-interface-sweep")
        .tempdir()
        .unwrap();
    let mut sources = vec![];
    for (name, json) in interfaces {
        let interface = parse_interface(PathBuf::from(format!("{name}.xir.json")), json)
            .unwrap_or_else(|e| panic!("`{name}` does not parse: {e:#}"));
        let text = xir_module_to_move_source(interface.module())
            .unwrap_or_else(|e| panic!("`{name}` does not lower to source: {e:#}"));
        let path = dir.path().join(format!("{}.move", name.replace("::", "_")));
        std::fs::write(&path, text).unwrap();
        sources.push(path.to_string_lossy().into_owned());
    }

    let count = sources.len();
    let originals = inline_bodies(interfaces.iter().map(|(name, json)| {
        parse_interface(PathBuf::from(format!("{name}.xir.json")), json)
            .unwrap()
            .module()
            .clone()
    }));
    let checked = run_checker(Options {
        sources,
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        // The framework's attributes (`resource_group`, `randomness`, …) are
        // Aptos-specific and unknown to the compiler's built-in set; a real
        // build passes them in. Attribute *checking* is not what is under test.
        skip_attribute_checks: true,
        // The same mapping the framework's `Move.toml` files declare, which a
        // real build always has.
        named_address_mapping: [
            "std=0x1",
            "vm=0x0",
            "vm_reserved=0x0",
            "aptos_std=0x1",
            "aptos_framework=0x1",
            "aptos_fungible_asset=0xA",
            "aptos_token=0x3",
            "aptos_token_objects=0x4",
            "core_resources=0xA550C18",
            "Extensions=0x1",
        ]
        .iter()
        .map(|entry| entry.to_string())
        .collect(),
        ..Options::default()
    })
    .unwrap_or_else(|e| {
        panic!(
            "`{}` generated interfaces failed to build: {:#}",
            package, e
        )
    });

    let mut diagnostics = Buffer::no_color();
    checked.report_diag(&mut diagnostics, Severity::Warning);
    assert!(
        !checked.has_errors(),
        "`{}` generated interfaces do not type-check:\n{}",
        package,
        String::from_utf8_lossy(&diagnostics.into_inner())
    );
    println!("{}: {} generated interfaces type-check", package, count);

    // A rendering that compiles can still mean something else. Exported again
    // from the recompiled modules, each inline body must render as it did: a
    // rendering that changed meaning on the way through reads back differently.
    let again = inline_bodies(
        checked
            .get_modules()
            .filter(|module| module.is_primary_target())
            .map(|module| {
                export_interface(&module).unwrap_or_else(|e| {
                    panic!("re-exporting `{}`: {e:#}", module.get_full_name_str())
                })
            }),
    );
    let mut changed = vec![];
    for (function, body) in &originals {
        match again.get(function) {
            Some(rendered) if normalized(rendered) == normalized(body) => {},
            Some(rendered) => changed.push(format!(
                "{function}:\n{body}\n-- reads back as --\n{rendered}"
            )),
            None => changed.push(format!("{function}: missing after recompilation")),
        }
    }
    assert!(
        changed.is_empty(),
        "`{}`: {} of {} inline bodies do not read back as rendered:\n{}",
        package,
        changed.len(),
        originals.len(),
        changed.join("\n\n")
    );
    println!(
        "{}: {} inline bodies read back as rendered",
        package,
        originals.len()
    );
}

/// `source` with what each rendering chooses afresh made uniform: temporaries
/// (`_t`, `_v0`, …) are named in order of appearance, and the braces and unit
/// statements the renderer prints around an otherwise unchanged statement are
/// dropped. What remains is compared exactly.
fn normalized(source: &str) -> String {
    let is_temporary = |word: &str| {
        let rest = word
            .strip_prefix("_t")
            .or_else(|| word.strip_prefix("_v"))
            .unwrap_or("x");
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '_') || word == "_t"
    };
    let mut temporaries = BTreeMap::new();
    let mut out = String::new();
    for line in source.lines() {
        let line = line.trim();
        if matches!(line, "{" | "}" | "};" | "();") {
            continue;
        }
        let mut word = String::new();
        for c in line.chars().chain(std::iter::once('\n')) {
            if c.is_ascii_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            if is_temporary(&word) {
                let next = temporaries.len();
                let canonical = temporaries.entry(word.clone()).or_insert(next);
                out.push_str(&format!("_tmp{canonical}"));
            } else {
                out.push_str(&word);
            }
            word.clear();
            out.push(c);
        }
    }
    out
}

/// The rendered inline bodies of `interfaces`, by `address::module::function`.
fn inline_bodies(
    interfaces: impl Iterator<Item = move_model_exchange::XirModule>,
) -> BTreeMap<String, String> {
    interfaces
        .flat_map(|interface| {
            let module = format!("{}::{}", interface.module.address, interface.module.name);
            interface.functions.into_iter().filter_map(move |function| {
                let source = function.source?;
                Some((format!("{module}::{}", function.name), source))
            })
        })
        .collect()
}

/// Both halves of the sweep, against one model of `package`.
///
/// They share a test because `build_model` type-checks the package and all of
/// its dependencies, which dominates the cost here — aptos-framework is 157
/// modules. Running the halves as separate tests builds that model twice.
///
/// It runs on a thread with a large stack: rendering an inline body recurses
/// with the depth of its expression.
fn sweep(package: &'static str) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let env = model_for(package);
            let interfaces = exported_interfaces(package, &env);
            check_round_trip(package, &interfaces);
            check_generated_interfaces(package, &interfaces);
        })
        .unwrap()
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
}

#[test]
fn move_stdlib_interfaces_export() {
    sweep("move-stdlib");
}

#[test]
fn aptos_stdlib_interfaces_export() {
    sweep("aptos-stdlib");
}

#[test]
fn aptos_framework_interfaces_export() {
    sweep("aptos-framework");
}
