// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use datatest_stable::Requirements;
use itertools::Itertools;
use log::debug;
use move_compiler_v2::{
    annotate_units, disassemble_compiled_units, env_pipeline::rewrite_target::RewritingScope,
    logging, pipeline, plan_builder, run_bytecode_verifier, run_file_format_gen, Experiment,
    Options,
};
use move_model::{metadata::LanguageVersion, model::GlobalEnv, sourcifier::Sourcifier};
use move_prover_test_utils::{baseline_test, extract_test_directives};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetPipeline;
use once_cell::unsync::Lazy;
use std::{
    cell::{RefCell, RefMut},
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Extension for expected output files
pub const EXP_EXT: &str = "exp";

/// Configuration for a set of tests.
#[derive(Clone)]
struct TestConfig {
    /// Name of this configuration
    name: &'static str,
    /// A static function pointer to the runner to be used for datatest. Since datatest
    /// does not support closures but only function pointers which can't capture
    /// context variables, we need to inject the config here. This MUST be set
    /// to `runner: |p| run_test(p, get_config_by_name("<name-of-this-config>"))`.
    /// See existing configurations before. NOTE: a common error is to use the
    /// wrong config name via copy & paste here, so watch out.
    runner: fn(&Path) -> datatest_stable::Result<()>,
    /// Path substring for tests to include.
    include: Vec<&'static str>,
    /// Path substring for tests to exclude. The set of tests included are those
    /// which match any of the include strings and do _not_ match any of
    /// the exclude strings.
    exclude: Vec<&'static str>,
    /// If set, a suffix for the baseline file used for these tests.
    /// If None, uses `exp`
    exp_suffix: Option<&'static str>,
    /// The options used for this run
    options: Options,
    /// After which step to stop processing
    stop_after: StopAfter,
    /// Determines what part of the AST to dump to .exp files
    dump_ast: DumpLevel,
    /// Determines what part of the stackless bytecode to dump to .exp files
    dump_bytecode: DumpLevel,
    /// If given, and `dump_bytecode == DumpLevel::AllStages`, restricts which stages
    /// to actual dump, by name of the processor.
    dump_bytecode_filter: Option<Vec<&'static str>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
enum DumpLevel {
    /// No dumping at all
    None,
    /// Only dump end stage of a pipeline
    EndStage,
    /// Dump all stages
    AllStages,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
enum StopAfter {
    /// Stop after the ast pipeline
    AstPipeline,
    /// Stop after bytecode generation
    BytecodeGen,
    /// Stop after bytecode pipeline runs to end (None) or to given processor.
    BytecodePipeline(Option<&'static str>),
    /// Run to the end, including file format generation and bytecode verification
    FileFormat,
}

/// Names for 'virtual' processors in the pipeline. This can be used for
/// filtering via the `config.dump_bytecode_filter` option.
const INITIAL_BYTECODE_STAGE: &str = "INITIAL_BYTECODE";
const FILE_FORMAT_STAGE: &str = "FILE_FORMAT";

/// Active test configurations. A test configuration is selected by
/// matching the include/exclude path specifications with the test file's path.
/// One test file can be selected for multiple configurations, in which case multiple
/// tests are generated for it. Each test file must have at least one matching
/// configuration.
///
/// Note: clippy would ask us to to turn `const` into `static` because interior mutable
/// constants are not safe in general. However, we can't because the `Options` type contains
/// a RefCell cache which does not implement Sync. In our use case, `const` should be safe
/// because this is the first time called by the single thread which collects the tests and
/// after that stays constant.
#[allow(clippy::declare_interior_mutable_const)]
const TEST_CONFIGS: Lazy<BTreeMap<&str, TestConfig>> = Lazy::new(|| {
    // Create default options
    let mut opts = Options::default()
        // Spec rewriter is always on, so we test it, even though it's not part of regular compiler.
        .set_experiment(Experiment::SPEC_REWRITE, true)
        // Turn optimization on by default. Some configs below may turn it off.
        .set_experiment(Experiment::OPTIMIZE, true)
        .set_experiment(Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true)
        .set_language_version(LanguageVersion::latest_stable());
    opts.testing = true;
    let configs = vec![
        // --- Tests for checking and ast processing
        // Tests for model building and various post-processing checking
        TestConfig {
            name: "checking",
            runner: |p| run_test(p, get_config_by_name("checking")),
            include: vec![
                "/checking/",
                "/parser/",
                "/visibility-checker/",
                "/cyclic-instantiation-checker/",
            ],
            // Need to exclude `inlining` because it is under checking
            // TODO: move `inlining` tests to top-level test directory
            exclude: vec!["/inlining/", "/more-v1/"],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::ACQUIRES_CHECK, false),
            stop_after: StopAfter::BytecodeGen, // FileFormat,
            dump_ast: DumpLevel::EndStage,
            dump_bytecode: DumpLevel::None, // EndStage,
            dump_bytecode_filter: None,
        },
        // Tests for checking v2 language features only supported if v2
        // language is selected
        TestConfig {
            name: "checking-lang-v1",
            runner: |p| run_test(p, get_config_by_name("checking-lang-v1")),
            include: vec!["/checking-lang-v1/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone().set_language_version(LanguageVersion::V1),
            stop_after: StopAfter::AstPipeline,
            dump_ast: DumpLevel::EndStage,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        TestConfig {
            name: "unused-assignment",
            runner: |p| run_test(p, get_config_by_name("unused-assignment")),
            include: vec!["/unused-assignment/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::UNUSED_ASSIGNMENT_CHECK, true),
            stop_after: StopAfter::BytecodePipeline(Some("UnusedAssignmentChecker")),
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        // Tests for lambda lifting and lambdas -- with full lambda support
        TestConfig {
            name: "lambda",
            runner: |p| run_test(p, get_config_by_name("lambda")),
            include: vec!["/lambda/", "/lambda-lifting/"],
            exclude: vec![],
            exp_suffix: Some("lambda.exp"),
            options: opts
                .clone()
                // .set_experiment(Experiment::AST_SIMPLIFY, true)
                .set_experiment(Experiment::LAMBDA_FIELDS, true)
                .set_experiment(Experiment::LAMBDA_IN_PARAMS, true)
                .set_experiment(Experiment::LAMBDA_IN_RETURNS, true)
                .set_experiment(Experiment::LAMBDA_VALUES, true)
                .set_experiment(Experiment::LAMBDA_LIFTING, true)
                .set_language_version(LanguageVersion::V2_LAMBDA),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::AllStages,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: None,
        },
        // Tests for simplifier in full mode, with code elimination
        TestConfig {
            name: "simplifier-full",
            runner: |p| run_test(p, get_config_by_name("simplifier-full")),
            include: vec!["/simplifier-elimination/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::AST_SIMPLIFY_FULL, true),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::EndStage,
            dump_bytecode: DumpLevel::None, // do not dump anything
            dump_bytecode_filter: None,
        },
        // Tests for more-v1 tests
        TestConfig {
            name: "more-v1",
            runner: |p| run_test(p, get_config_by_name("more-v1")),
            include: vec!["/more-v1/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone().set_experiment(Experiment::AST_SIMPLIFY, true),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None, // do not dump anything
            dump_bytecode_filter: None,
        },
        // Tests for inlining, simplifier, and folding
        TestConfig {
            name: "inlining-et-al",
            runner: |p| run_test(p, get_config_by_name("inlining-et-al")),
            include: vec!["/inlining/", "/folding/", "/simplifier/", "/lambda/"],
            exclude: vec!["/more-v1/"],
            exp_suffix: None,
            options: opts.clone().set_experiment(Experiment::AST_SIMPLIFY, true),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::EndStage,
            dump_bytecode: DumpLevel::None, // do not dump anything
            dump_bytecode_filter: None,
        },
        // Tests for targets in non-simplifier
        TestConfig {
            name: "no-simplifier",
            runner: |p| run_test(p, get_config_by_name("no-simplifier")),
            include: vec!["/no-simplifier/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone().set_experiment(Experiment::AST_SIMPLIFY, false),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::EndStage,
            dump_bytecode: DumpLevel::None, // do not dump anything
            dump_bytecode_filter: None,
        },
        // Tests for diagnostics, where dumping AST isn't useful.
        TestConfig {
            name: "diagnostics",
            runner: |p| run_test(p, get_config_by_name("diagnostics")),
            include: vec!["/deprecated/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone().set_experiment(Experiment::AST_SIMPLIFY, true),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None, // do not dump anything
            dump_bytecode_filter: None,
        },
        // --- Tests for bytecode generation
        TestConfig {
            name: "bytecode-gen",
            runner: |p| run_test(p, get_config_by_name("bytecode-gen")),
            include: vec!["/bytecode-generator/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::EndStage,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: Some(vec![INITIAL_BYTECODE_STAGE]),
        },
        // -- Tests for stages in the bytecode pipeline
        // Live-var tests
        TestConfig {
            name: "live-var",
            runner: |p| run_test(p, get_config_by_name("live-var")),
            include: vec!["/live-var/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            stop_after: StopAfter::BytecodePipeline(Some("LiveVarAnalysisProcessor")),
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![INITIAL_BYTECODE_STAGE, "LiveVarAnalysisProcessor"]),
        },
        // Reference safety tests, old version (with optimizations on)
        TestConfig {
            name: "reference-safety-old",
            runner: |p| run_test(p, get_config_by_name("reference-safety-old")),
            include: vec!["/reference-safety/"],
            exclude: vec![],
            exp_suffix: Some("old.exp"),
            // TODO(#13485): Need to turn off acquires check for now to test 2.0 access specifiers
            options: opts.clone().set_experiment(Experiment::ACQUIRES_CHECK, false).
                set_experiment(Experiment::REFERENCE_SAFETY_V3, false),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter:
            // For debugging (dump_bytecode set DumpLevel::AllStages)
            Some(vec![
                INITIAL_BYTECODE_STAGE,
                "ReferenceSafetyProcessor",
                "DeadStoreElimination",
                FILE_FORMAT_STAGE,
            ]),
        },
        // Reference safety tests (with optimizations on)
        TestConfig {
            name: "reference-safety",
            runner: |p| run_test(p, get_config_by_name("reference-safety")),
            include: vec!["/reference-safety/"],
            exclude: vec![],
            // Some reference tests create different errors since variable names are
            // known without optimizations, so we need to have a different exp file
            exp_suffix: None,
            options: opts.clone().set_experiment(Experiment::REFERENCE_SAFETY_V3, true)
                .set_experiment(Experiment::ACQUIRES_CHECK, false),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter:
            // For debugging:
            Some(vec![
                INITIAL_BYTECODE_STAGE,
                "ReferenceSafetyProcessor",
                "DeadStoreElimination",
                FILE_FORMAT_STAGE,
            ]),
        },
        // Reference safety tests no-opt
        TestConfig {
            name: "reference-safety-no-opt",
            runner: |p| run_test(p, get_config_by_name("reference-safety-no-opt")),
            include: vec!["/reference-safety/"],
            exclude: vec![],
            // Some reference tests create different errors since variable names are
            // known without optimizations, so we need to have a different exp file
            exp_suffix: Some("no-opt.exp"),
            options: opts.clone().set_experiment(Experiment::OPTIMIZE, false)
                .set_experiment(Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, false)
                .set_experiment(Experiment::ACQUIRES_CHECK, false),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter:
            // For debugging:
            Some(vec![
                INITIAL_BYTECODE_STAGE,
                "ReferenceSafetyProcessor",
                "AbilityProcessor",
                "DeadStoreElimination",
                FILE_FORMAT_STAGE,
            ]),
        },
        // Abort analysis tests
        TestConfig {
            name: "abort-analysis",
            runner: |p| run_test(p, get_config_by_name("abort-analysis")),
            include: vec!["/abort-analysis/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            stop_after: StopAfter::BytecodePipeline(Some("AbortAnalysisProcessor")),
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![INITIAL_BYTECODE_STAGE, "AbortAnalysisProcessor"]),
        },
        // Ability checking tests
        TestConfig {
            name: "ability-check",
            runner: |p| run_test(p, get_config_by_name("ability-check")),
            include: vec!["/ability-check/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        // Ability transformation tests
        TestConfig {
            name: "ability-transform",
            runner: |p| run_test(p, get_config_by_name("ability-transform")),
            include: vec!["/ability-transform/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            stop_after: StopAfter::BytecodePipeline(Some("AbilityProcessor")),
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![
                INITIAL_BYTECODE_STAGE,
                "AbortAnalysisProcessor",
                "LiveVarAnalysisProcessor",
                "ReferenceSafetyProcessor",
                "AbilityProcessor",
            ]),
        },
        TestConfig {
            name: "acquires-checker",
            runner: |p| run_test(p, get_config_by_name("acquires-checker")),
            include: vec!["/acquires-checker/"],
            exclude: vec![],
            exp_suffix: None,
            // Skip access check to avoid error message change in the acquires-checker
            options: opts.clone().set_experiment(Experiment::ACCESS_CHECK, false),
            // Run the full compiler pipeline to double-check the result.
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        // Bytecode verifier tests
        TestConfig {
            name: "bytecode-verify",
            runner: |p| run_test(p, get_config_by_name("bytecode-verify")),
            include: vec!["/bytecode-verify-failure/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                // Note that we do not run ability checker here, as we want to induce
                // a bytecode verification failure. The test in /bytecode-verify-failure/
                // has erroneous ability annotations.
                .set_experiment(Experiment::ABILITY_CHECK, false),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: Some(vec!["FILE_FORMAT"]),
        },
        // Copy propagation
        TestConfig {
            name: "copy-propagation",
            runner: |p| run_test(p, get_config_by_name("copy-propagation")),
            include: vec!["/copy-propagation/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::VARIABLE_COALESCING, false)
                .set_experiment(Experiment::COPY_PROPAGATION, true),
            stop_after: StopAfter::BytecodePipeline(Some("DeadStoreElimination")),
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![
                INITIAL_BYTECODE_STAGE,
                "AvailableCopiesAnalysisProcessor",
                "CopyPropagation",
                "DeadStoreElimination",
            ]),
        },
        // Variable coalescing tests
        TestConfig {
            name: "variable-coalescing",
            runner: |p| run_test(p, get_config_by_name("variable-coalescing")),
            include: vec!["/variable-coalescing/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                // Turn off simplification
                .set_experiment(Experiment::AST_SIMPLIFY, false)
                // For testing
                .set_experiment(Experiment::VARIABLE_COALESCING_ANNOTATE, true),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![
                INITIAL_BYTECODE_STAGE,
                "VariableCoalescingAnnotator",
                "VariableCoalescingTransformer",
                "DeadStoreElimination",
                FILE_FORMAT_STAGE,
            ]),
        },
        // Variable coalescing tests w/ optimizations
        TestConfig {
            name: "variable-coalescing-opt",
            runner: |p| run_test(p, get_config_by_name("variable-coalescing-opt")),
            include: vec!["/variable-coalescing/"],
            exclude: vec![],
            exp_suffix: Some("opt.exp"),
            options: opts
                .clone()
                // For testing
                .set_experiment(Experiment::VARIABLE_COALESCING_ANNOTATE, true),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![
                INITIAL_BYTECODE_STAGE,
                "VariableCoalescingAnnotator",
                "VariableCoalescingTransformer",
                "DeadStoreElimination",
                FILE_FORMAT_STAGE,
            ]),
        },
        // Flush writes processor tests
        TestConfig {
            name: "flush-writes-on",
            runner: |p| run_test(p, get_config_by_name("flush-writes-on")),
            include: vec!["/flush-writes/"],
            exclude: vec![],
            exp_suffix: Some("on.exp"),
            options: opts
                .clone()
                .set_experiment(Experiment::FLUSH_WRITES_OPTIMIZATION, true),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec!["FlushWritesProcessor", FILE_FORMAT_STAGE]),
        },
        TestConfig {
            name: "flush-writes-off",
            runner: |p| run_test(p, get_config_by_name("flush-writes-off")),
            include: vec!["/flush-writes/"],
            exclude: vec![],
            exp_suffix: Some("off.exp"),
            options: opts
                .clone()
                .set_experiment(Experiment::FLUSH_WRITES_OPTIMIZATION, false),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![FILE_FORMAT_STAGE]),
        },
        // Unreachable code remover
        TestConfig {
            name: "unreachable-code",
            runner: |p| run_test(p, get_config_by_name("unreachable-code")),
            include: vec!["/unreachable-code-remover/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![
                INITIAL_BYTECODE_STAGE,
                "UnreachableCodeProcessor",
                "UnreachableCodeRemover",
            ]),
        },
        // Uninitialized use checker
        TestConfig {
            name: "uninit-use",
            runner: |p| run_test(p, get_config_by_name("uninit-use")),
            include: vec!["/uninit-use-checker/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::KEEP_UNINIT_ANNOTATIONS, true),
            stop_after: StopAfter::BytecodePipeline(Some("uninitialized_use_checker")),
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec![INITIAL_BYTECODE_STAGE, "uninitialized_use_checker"]),
        },
        // -- File Format Generation
        // Test without bytecode optimizations enabled
        TestConfig {
            name: "file-format",
            runner: |p| run_test(p, get_config_by_name("file-format")),
            include: vec!["/file-format-generator/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::OPTIMIZE, false)
                .set_experiment(Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, false)
                .set_experiment(Experiment::AST_SIMPLIFY, true),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: Some(vec![FILE_FORMAT_STAGE]),
        },
        // Test with bytecode optimizations enabled
        TestConfig {
            name: "file-format-opt",
            runner: |p| run_test(p, get_config_by_name("file-format-opt")),
            include: vec!["/file-format-generator/"],
            exclude: vec![],
            exp_suffix: Some("opt.exp"),
            options: opts.clone().set_experiment(Experiment::OPTIMIZE, true),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: Some(vec![FILE_FORMAT_STAGE]),
        },
        // Test for unit tests on and off
        TestConfig {
            name: "unit-test-on",
            runner: |p| run_test(p, get_config_by_name("unit-test-on")),
            include: vec!["/unit_test/test/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::AST_SIMPLIFY, true)
                .set_compile_test_code(true),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        TestConfig {
            name: "unit-test-off",
            runner: |p| run_test(p, get_config_by_name("unit-test-off")),
            include: vec!["/unit_test/notest/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::AST_SIMPLIFY, true)
                .set_compile_test_code(false),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        // Test for verify on and off
        TestConfig {
            name: "verification",
            runner: |p| run_test(p, get_config_by_name("verification")),
            include: vec!["/verification/verify/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::AST_SIMPLIFY, true)
                .set_compile_test_code(true),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        TestConfig {
            name: "verification-off",
            runner: |p| run_test(p, get_config_by_name("verification-off")),
            include: vec!["/verification/noverify/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::AST_SIMPLIFY, true)
                .set_compile_test_code(false),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        TestConfig {
            name: "skip-attribute-checks",
            runner: |p| run_test(p, get_config_by_name("skip-attribute-checks")),
            include: vec!["/skip_attribute_checks/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts
                .clone()
                .set_experiment(Experiment::AST_SIMPLIFY, true)
                .set_skip_attribute_checks(true),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::None,
            dump_bytecode_filter: None,
        },
        TestConfig {
            name: "control-flow-simplification-on",
            runner: |p| run_test(p, get_config_by_name("control-flow-simplification-on")),
            include: vec!["/control-flow-simplification/"],
            exclude: vec![],
            exp_suffix: Some("on.exp"),
            options: opts
                .clone()
                .set_experiment(Experiment::CFG_SIMPLIFICATION, true),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::AllStages,
            dump_bytecode_filter: Some(vec!["ControlFlowGraphSimplifier", FILE_FORMAT_STAGE]),
        },
        TestConfig {
            name: "control-flow-simplification-off",
            runner: |p| run_test(p, get_config_by_name("control-flow-simplification-off")),
            include: vec!["/control-flow-simplification/"],
            exclude: vec![],
            exp_suffix: Some("off.exp"),
            options: opts
                .clone()
                .set_experiment(Experiment::CFG_SIMPLIFICATION, false),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: Some(vec![FILE_FORMAT_STAGE]),
        },
        TestConfig {
            name: "op-equal",
            runner: |p| run_test(p, get_config_by_name("op-equal")),
            include: vec!["/op-equal/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            // Run the entire compiler pipeline to double-check the result
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::EndStage,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: None,
        },
        TestConfig {
            name: "eager-pushes",
            runner: |p| run_test(p, get_config_by_name("eager-pushes")),
            include: vec!["/eager-pushes/"],
            exclude: vec![],
            exp_suffix: None,
            options: opts.clone(),
            stop_after: StopAfter::FileFormat,
            dump_ast: DumpLevel::None,
            dump_bytecode: DumpLevel::EndStage,
            dump_bytecode_filter: None,
        },
    ];
    configs.into_iter().map(|c| (c.name, c)).collect()
});

/// A function which gets a copy of a TestConfig by name.
#[allow(clippy::borrow_interior_mutable_const)]
fn get_config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("wrongly named test config `{}`", name))
}

/// Runs test at `path` with the given `config`.
fn run_test(path: &Path, config: TestConfig) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing();
    let path_str = path.display().to_string();
    let mut options = config.options.clone();
    options.warn_unused = path_str.contains("/unused/");
    options.warn_deprecated = path_str.contains("/deprecated/");
    options.compile_verify_code = path_str.contains("/verification/verify/");
    options.sources_deps = extract_test_directives(path, "// dep:")?;
    options.sources = vec![path_str.clone()];
    options.dependencies = if extract_test_directives(path, "// no-stdlib")?.is_empty() {
        vec![path_from_crate_root("../move-stdlib/sources")]
    } else {
        vec![]
    };
    options.named_address_mapping = vec![
        "std=0x1".to_string(),
        "aptos_std=0x1".to_string(),
        "M=0x1".to_string(),
        "A=0x42".to_string(),
        "B=0x42".to_string(),
        "K=0x19".to_string(),
    ];

    // Putting the generated test baseline into a Refcell to avoid problems with mut borrow
    // in closures.
    let test_output = RefCell::new(String::new());

    // Run context checker
    let mut env = move_compiler_v2::run_checker(options.clone())?;
    let mut ok = check_diags(&mut test_output.borrow_mut(), &env);

    if ok {
        // Run env processor pipeline.
        let env_pipeline = move_compiler_v2::check_and_rewrite_pipeline(
            &options,
            false,
            RewritingScope::CompilationTarget,
        );
        if config.dump_ast == DumpLevel::AllStages {
            let mut out = Buffer::no_color();

            env_pipeline.run_and_record(&mut env, &mut out)?;
            test_output
                .borrow_mut()
                .push_str(&String::from_utf8_lossy(&out.into_inner()));
            ok = check_diags(&mut test_output.borrow_mut(), &env);
        } else {
            env_pipeline.run(&mut env);
            ok = check_diags(&mut test_output.borrow_mut(), &env);
            if ok && config.dump_ast == DumpLevel::EndStage {
                test_output.borrow_mut().push_str(&format!(
                    "// -- Model dump before bytecode pipeline\n{}\n",
                    env.dump_env()
                ));
                let sourcifier = Sourcifier::new(&env);
                for module in env.get_modules() {
                    if module.is_primary_target() {
                        sourcifier.print_module(module.get_id())
                    }
                }
                test_output.borrow_mut().push_str(&format!(
                    "// -- Sourcified model before bytecode pipeline\n{}\n",
                    sourcifier.result()
                ));
            }
        }
    }

    if ok && options.compile_test_code {
        // Build the test plan here to parse and validate any test-related attributes in the AST.
        // In real use, this is run outside of the compilation process, but the needed info is
        // available in `env` once we finish the AST.
        plan_builder::construct_test_plan(&env, None);
        ok = check_diags(&mut test_output.borrow_mut(), &env);
    }

    if ok && config.stop_after > StopAfter::AstPipeline {
        // Run stackless bytecode generator
        let mut targets = move_compiler_v2::run_bytecode_gen(&env);
        ok = check_diags(&mut test_output.borrow_mut(), &env);
        if ok {
            // Run the target pipeline.
            let bytecode_pipeline = if config.stop_after == StopAfter::BytecodeGen {
                // Empty pipeline -- just use it for dumping the result of bytecode gen
                FunctionTargetPipeline::default()
            } else {
                let mut pipeline = move_compiler_v2::bytecode_pipeline(&env);
                if let StopAfter::BytecodePipeline(Some(processor)) = &config.stop_after {
                    pipeline.stop_after_for_testing(processor)
                }
                pipeline
            };
            let count = bytecode_pipeline.processor_count();
            let ok = RefCell::new(true);
            bytecode_pipeline.run_with_hook(
                &env,
                &mut targets,
                // Hook which is run before steps in the pipeline. Prints out initial
                // bytecode from the generator, if requested.
                |targets_before| {
                    let out = &mut test_output.borrow_mut();
                    update_diags(ok.borrow_mut(), out, &env);
                    if bytecode_dump_enabled(&config, true, INITIAL_BYTECODE_STAGE) {
                        let dump =
                            &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                &env,
                                "initial bytecode",
                                targets_before,
                                &pipeline::register_formatters,
                                false,
                            );
                        out.push_str(dump);
                        debug!("{}", dump)
                    }
                },
                // Hook which is run after every step in the pipeline. Prints out
                // bytecode after the processor, if requested.
                |i, processor, targets_after| {
                    let out = &mut test_output.borrow_mut();
                    update_diags(ok.borrow_mut(), out, &env);
                    if bytecode_dump_enabled(&config, i + 1 == count, processor.name().as_str()) {
                        let title = format!("after {}:", processor.name());
                        let dump =
                            &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                &env,
                                &title,
                                targets_after,
                                &pipeline::register_formatters,
                                false,
                            );
                        out.push_str(dump);
                        debug!("{}", dump);
                    }
                    *ok.borrow()
                },
            );
            if *ok.borrow() && config.stop_after == StopAfter::FileFormat {
                let units = run_file_format_gen(&mut env, &targets);
                let out = &mut test_output.borrow_mut();
                update_diags(ok.borrow_mut(), out, &env);
                if *ok.borrow() {
                    if bytecode_dump_enabled(&config, true, FILE_FORMAT_STAGE) {
                        out.push_str(
                            "\n============ disassembled file-format ==================\n",
                        );
                        out.push_str(&disassemble_compiled_units(&units)?);
                    }
                    let annotated_units = annotate_units(units);
                    if run_bytecode_verifier(&annotated_units, &mut env) {
                        out.push_str("\n============ bytecode verification succeeded ========\n");
                    } else {
                        out.push_str("\n============ bytecode verification failed ========\n");
                    }
                    check_diags(out, &env);
                }
            }
        }
    }

    // Generate/check baseline.
    let exp_file_ext = config.exp_suffix.unwrap_or("exp");
    let baseline_path = path.with_extension(exp_file_ext);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &test_output.borrow())?;
    Ok(())
}

fn bytecode_dump_enabled(config: &TestConfig, is_last: bool, name: &str) -> bool {
    (config.dump_bytecode == DumpLevel::AllStages
        || config.dump_bytecode == DumpLevel::EndStage && is_last)
        && (config.dump_bytecode_filter.is_none()
            || config
                .dump_bytecode_filter
                .as_ref()
                .unwrap()
                .contains(&name))
}

/// Checks for diagnostics and adds them to the baseline.
fn check_diags(baseline: &mut String, env: &GlobalEnv) -> bool {
    let mut error_writer = Buffer::no_color();
    env.report_diag(&mut error_writer, Severity::Note);
    let diag = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    if !diag.is_empty() {
        *baseline += &format!("\nDiagnostics:\n{}", diag);
    }
    let ok = !env.has_errors();
    env.clear_diag();
    ok
}

fn update_diags(mut ok: RefMut<bool>, baseline: &mut String, env: &GlobalEnv) {
    if !check_diags(baseline, env) {
        *ok = false;
    }
}

/// Returns a path relative to the crate root.
fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}

/// Collects tests found under the given root and adds them to a list of
/// `datatest_stable::Requirements`, where each entry represents one `(path, config)`
/// combination. This errors if not configuration is found for a given test path.
/// TODO: we may want to add some filters here based on env vars (like the prover does),
///    which allow e.g. to filter by configuration name.
#[allow(clippy::borrow_interior_mutable_const)]
fn collect_tests(root: &str) -> Vec<Requirements> {
    let mut test_groups: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .flatten()
    {
        let entry_str = entry.path().to_string_lossy().to_string();
        if !entry_str.ends_with(".move") {
            continue;
        }
        let mut found_one = false;
        for config in TEST_CONFIGS.values() {
            if !config.include.iter().any(|s| entry_str.contains(s))
                || config.exclude.iter().any(|s| entry_str.contains(s))
            {
                // no match
                continue;
            }
            test_groups
                .entry(config.name)
                .or_default()
                .push(entry_str.clone());
            found_one = true
        }
        assert!(
            found_one,
            "cannot find test configuration for `{}`",
            entry_str
        )
    }
    let mut reqs = vec![];
    for (name, files) in test_groups {
        let config = get_config_by_name(name);
        reqs.push(Requirements::new(
            config.runner,
            // This will appear in the output of cargo test/nextest
            format!("compiler-v2[config={}]", config.name),
            root.to_string(),
            files.into_iter().map(|s| s + "$").join("|"),
        ));
    }
    reqs
}

fn main() {
    let reqs = collect_tests("tests");
    datatest_stable::runner(&reqs)
}
