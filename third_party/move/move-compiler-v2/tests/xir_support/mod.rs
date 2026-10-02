// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared fixtures and drivers for the XIR reader tests.
//!
//! These drive whole programs through the compiler rather than exercising one
//! function, so they live beside the integration tests that use them rather
//! than inside `xir.rs`.

#![allow(dead_code)]

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_compiler_v2::{
    run_checker, run_file_format_gen, run_stackless_bytecode_pipeline,
    stackless_bytecode_check_pipeline, stackless_bytecode_optimization_pipeline,
    xir::{import_sources, parse_source},
    Options,
};
use move_model::{
    metadata::{CompilerVersion, LanguageVersion},
    model::GlobalEnv,
};
use move_model_exchange::XirModule;
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use std::{fs, path::PathBuf};

/// The committed XIR document used as a starting point by several tests.
pub fn account_golden() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xir/account.xir.json"))
        .unwrap()
}

pub fn account_module() -> XirModule {
    serde_json::from_str(&account_golden()).unwrap()
}

/// A model holding the Move standard library.
///
/// Vector operations lower to calls into `0x1::vector` — `push_back`,
/// `borrow`, `swap` and the rest — so the reader needs that module present
/// before it can translate any of them.
pub fn stdlib_options() -> Options {
    Options {
        // As dependencies, not targets: the XIR module under test must be
        // the only primary target, or the stackless pipeline expects
        // function targets for all of the standard library too.
        dependencies: move_stdlib::move_stdlib_files(),
        // The sources say `module std::vector`, so the name must be bound.
        named_address_mapping: vec!["std=0x1".to_owned()],
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        ..Options::default()
    }
}

pub fn env_with_stdlib(options: &Options) -> GlobalEnv {
    let env = run_checker(options.clone()).expect("the standard library models");
    assert!(
        !env.has_errors() && env.get_modules().any(|m| m.is_std_vector()),
        "the standard library did not model; {} modules",
        env.get_module_count()
    );
    env
}

pub fn report_if_errors(env: &GlobalEnv, stage: &str) {
    if env.has_errors() {
        let mut out = Buffer::no_color();
        env.report_diag(&mut out, Severity::Error);
        panic!(
            "{stage} reported:\n{}",
            String::from_utf8_lossy(&out.into_inner())
        );
    }
}

/// Imports `module` into an env that already holds its dependencies, and
/// drives it to verified bytecode.
///
/// Reports diagnostics at each stage: an error left unreported here
/// surfaces much later as a missing annotation inside the file-format
/// generator, which names neither the stage nor the cause.
pub fn verify_xir_against(env: &mut GlobalEnv, options: &Options, module: XirModule, path: &str) {
    let source = parse_source(
        PathBuf::from(path),
        String::new(),
        &serde_json::to_string(&module).unwrap(),
    )
    .unwrap();
    let mut targets = FunctionTargetsHolder::default();
    import_sources(env, &[source], &mut targets).expect("the module loads");
    report_if_errors(env, "importing");

    env.set_extension(options.clone());
    run_stackless_bytecode_pipeline(
        env,
        stackless_bytecode_check_pipeline(options),
        &mut targets,
    );
    report_if_errors(env, "the stackless checks");
    run_stackless_bytecode_pipeline(
        env,
        stackless_bytecode_optimization_pipeline(options),
        &mut targets,
    );
    report_if_errors(env, "the optimization pipeline");
    let units = run_file_format_gen(env, &targets);
    report_if_errors(env, "file format generation");
    let legacy_move_compiler::compiled_unit::CompiledUnit::Module(unit) = &units[0] else {
        panic!("expected a module")
    };
    move_bytecode_verifier::verify_module(&unit.module).expect("the generated code verifies");
}
