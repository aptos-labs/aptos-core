// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Defines 'experiments' (flags) for the compiler. Most phases of the
//! compiler can be enabled or disabled via an experiment. An experiment
//! can be set via the command line (`--experiment name[=on/off]`),
//! via an environment variable (`MVC_EXP=def,..` where `def` is
//! `name[=on/off]`), or programmatically. Experiments are retrieved
//! via `options.experiment_on(NAME)`.
//!
//! The declaration of experiments happens via the datatype `Experiment`
//! which defines its name, description, and default value. The default
//! can be either be a fixed constant or inherited from another
//! experiment, effectively allowing to activate a group of experiments
//! via some meta-experiment. For example, the `OPTIMIZE` experiment
//! turns on or off a bunch of other experiments, unless those are
//! defined explicitly.

use once_cell::sync::Lazy;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Experiment {
    /// The name of the experiment
    pub name: String,
    /// A description of the experiment
    pub description: String,
    /// Whether the default is true or false
    pub default: DefaultValue,
}

#[derive(Clone)]
pub enum DefaultValue {
    /// Whether the default is a fixed value.
    Given(bool),
    /// Whether the default is inherited from another experiment
    Inherited(String),
}

pub static EXPERIMENTS: Lazy<BTreeMap<String, Experiment>> = Lazy::new(|| {
    use DefaultValue::*;
    let experiments = vec![
        Experiment {
            name: Experiment::CHECKS.to_string(),
            description: "Turns on or off a group of context checks".to_string(),
            default: Given(true),
        },
        Experiment {
            name: Experiment::REFERENCE_SAFETY.to_string(),
            description: "Turns on or off reference safety check error reporting".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::REFERENCE_SAFETY_V3.to_string(),
            description: "Turns on or off whether to use the new v3 reference safety checker"
                .to_string(),
            default: Given(true), // v3 is the default
        },
        Experiment {
            name: Experiment::USAGE_CHECK.to_string(),
            description: "Turns on or off checks for correct usage of types and variables"
                .to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::UNINITIALIZED_CHECK.to_string(),
            description: "Turns on or off checks for uninitialized variables".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::KEEP_UNINIT_ANNOTATIONS.to_string(),
            description: "Determines whether the annotations for \
            uninitialized variable analysis should be kept around (for testing)"
                .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::ABILITY_CHECK.to_string(),
            description: "Turns on or off ability checks".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::ACCESS_CHECK.to_string(),
            description: "Turns on or off access and use checks".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::ACQUIRES_CHECK.to_string(),
            description: "Turns on or off v1 style acquires checks".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::SEQS_IN_BINOPS_CHECK.to_string(),
            description: "Turns on or off checks for sequences within binary operations"
                .to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::CMP_REWRITE.to_string(),
            description: "Rewrite comparison operations".to_string(),
            default: Given(true),
        },
        Experiment {
            name: Experiment::SIGNED_INT_REWRITE.to_string(),
            description: "Rewrite signed integer types and operations".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::INLINING.to_string(),
            description: "Turns on or off inlining".to_string(),
            default: Given(true),
        },
        Experiment {
            name: Experiment::INLINING_OPTIMIZATION.to_string(),
            description: "Turns on or off inlining optimizations".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::ACROSS_PACKAGE_INLINING.to_string(),
            description: "Turns on or off inlining across package boundaries".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::SPEC_CHECK.to_string(),
            description: "Turns on or off specification checks".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::SPEC_REWRITE.to_string(),
            description: "Turns on or off specification rewriting".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::LAMBDA_LIFTING_INLINE.to_string(),
            description: "Whether inline functions shall be included in lambda lifting".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::RECURSIVE_TYPE_CHECK.to_string(),
            description: "Turns on or off checking of recursive structs and type instantiations"
                .to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::SPLIT_CRITICAL_EDGES.to_string(),
            description: "Turns on or off splitting of critical edges".to_string(),
            default: Given(true),
        },
        Experiment {
            name: Experiment::OPTIMIZE.to_string(),
            description: "Turns on standard group of optimizations".to_string(),
            default: Given(true),
        },
        Experiment {
            name: Experiment::OPTIMIZE_EXTRA.to_string(),
            description: "Use extra optimizations".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS.to_string(),
            description: "Turns on optimizations waiting for comparison testing".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::CFG_SIMPLIFICATION.to_string(),
            description: "Whether to do the control flow graph simplification".to_string(),
            default: Inherited(Experiment::OPTIMIZE.to_string()),
        },
        Experiment {
            name: Experiment::DEAD_CODE_ELIMINATION.to_string(),
            description: "Whether to run dead store and unreachable code elimination".to_string(),
            default: Inherited(Experiment::OPTIMIZE.to_string()),
        },
        Experiment {
            name: Experiment::PEEPHOLE_OPTIMIZATION.to_string(),
            description: "Whether to run peephole optimization on generated file format"
                .to_string(),
            default: Inherited(Experiment::OPTIMIZE.to_string()),
        },
        Experiment {
            name: Experiment::UNUSED_STRUCT_PARAMS_CHECK.to_string(),
            description: "Whether to check for unused struct type parameters".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::UNUSED_ASSIGNMENT_CHECK.to_string(),
            description: "Whether to check for unused assignments".to_string(),
            default: Inherited(Experiment::CHECKS.to_string()),
        },
        Experiment {
            name: Experiment::VARIABLE_COALESCING.to_string(),
            description: "Whether to run variable coalescing".to_string(),
            default: Inherited(Experiment::OPTIMIZE.to_string()),
        },
        Experiment {
            name: Experiment::VARIABLE_COALESCING_ANNOTATE.to_string(),
            description: "Whether to run variable coalescing, annotation only (for testing)"
                .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::KEEP_INLINE_FUNS.to_string(),
            description: "Whether to keep functions after inlining \
            or remove them from the model"
                .to_string(),
            default: Given(true),
        },
        Experiment {
            name: Experiment::LIFT_INLINE_FUNS.to_string(),
            description: "Whether to lift lambda expressions passed to inline functions"
                .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::SKIP_INLINING_INLINE_FUNS.to_string(),
            description: "Whether to skip inlining the (standalone) inline functions".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::AST_SIMPLIFY.to_string(),
            description: "Whether to run the ast simplifier".to_string(),
            default: Inherited(Experiment::OPTIMIZE.to_string()),
        },
        Experiment {
            name: Experiment::AST_SIMPLIFY_FULL.to_string(),
            description: "Whether to run the ast simplifier, including code elimination"
                .to_string(),
            default: Inherited(Experiment::OPTIMIZE_EXTRA.to_string()),
        },
        Experiment {
            name: Experiment::ATTACH_COMPILED_MODULE.to_string(),
            description: "Whether to attach the compiled module to the global env.".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::LINT_CHECKS.to_string(),
            description: "Whether to run various lint checks.".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::FLUSH_WRITES_OPTIMIZATION.to_string(),
            description: "Whether to run flush writes processor and optimization".to_string(),
            default: Inherited(Experiment::OPTIMIZE.to_string()),
        },
        Experiment {
            name: Experiment::STOP_BEFORE_STACKLESS_BYTECODE.to_string(),
            description:
                "Exit quietly just before converting to stackless bytecode (after AST passes)"
                    .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::STOP_BEFORE_FILE_FORMAT.to_string(),
            description:
                "Exit quietly just before generating file format (after stackless bytecode  passes)"
                    .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::STOP_BEFORE_EXTENDED_CHECKS.to_string(),
            description: "Exit quietly just before extended checks (after file format generation)"
                .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::STOP_AFTER_EXTENDED_CHECKS.to_string(),
            description: "Exit quietly just after extended checks (after file format generation)"
                .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::MESSAGE_FORMAT_JSON.to_string(),
            description: "Enable json format for compiler messages".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::FAIL_ON_WARNING.to_string(),
            description: "Fail compilation if there is a warning".to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::SKIP_BAILOUT_ON_EXTENDED_CHECKS.to_string(),
            description: "Skip errors originating from extended checks, for testing only"
                .to_string(),
            default: Given(false),
        },
        Experiment {
            name: Experiment::UNSAFE_PACKAGE_VISIBILITY.to_string(),
            description: "Treat all package functions with same address as visible (currently necessary for prover in filter mode)"
                .to_string(),
            default: Given(false),
        },
    ];
    experiments
        .into_iter()
        .map(|e| (e.name.clone(), e))
        .collect()
});

/// For documentation of the constants here, see the definition of `EXPERIMENTS`.
impl Experiment {
    pub const ABILITY_CHECK: &'static str = "ability-check";
    pub const ACCESS_CHECK: &'static str = "access-use-function-check";
    pub const ACQUIRES_CHECK: &'static str = "acquires-check";
    pub const ACROSS_PACKAGE_INLINING: &'static str = "across-package-inlining";
    pub const AST_SIMPLIFY: &'static str = "ast-simplify";
    pub const AST_SIMPLIFY_FULL: &'static str = "ast-simplify-full";
    pub const ATTACH_COMPILED_MODULE: &'static str = "attach-compiled-module";
    pub const CFG_SIMPLIFICATION: &'static str = "cfg-simplification";
    pub const CHECKS: &'static str = "checks";
    pub const CMP_REWRITE: &'static str = "cmp-rewrite";
    pub const DEAD_CODE_ELIMINATION: &'static str = "dead-code-elimination";
    pub const DUPLICATE_STRUCT_PARAMS_CHECK: &'static str = "duplicate-struct-params-check";
    pub const FAIL_ON_WARNING: &'static str = "fail-on-warning";
    pub const FLUSH_WRITES_OPTIMIZATION: &'static str = "flush-writes-optimization";
    pub const INLINING: &'static str = "inlining";
    pub const INLINING_OPTIMIZATION: &'static str = "inlining-optimization";
    pub const KEEP_INLINE_FUNS: &'static str = "keep-inline-funs";
    pub const KEEP_UNINIT_ANNOTATIONS: &'static str = "keep-uninit-annotations";
    pub const LAMBDA_LIFTING_INLINE: &'static str = "lambda-lifting-inline";
    pub const LIFT_INLINE_FUNS: &'static str = "lift-inline-funs";
    pub const LINT_CHECKS: &'static str = "lint-checks";
    pub const MESSAGE_FORMAT_JSON: &'static str = "compiler-message-format-json";
    pub const OPTIMIZE: &'static str = "optimize";
    pub const OPTIMIZE_EXTRA: &'static str = "optimize-extra";
    pub const OPTIMIZE_WAITING_FOR_COMPARE_TESTS: &'static str =
        "optimize-waiting-for-compare-tests";
    pub const PEEPHOLE_OPTIMIZATION: &'static str = "peephole-optimization";
    pub const RECURSIVE_TYPE_CHECK: &'static str = "recursive-type-check";
    pub const REFERENCE_SAFETY: &'static str = "reference-safety";
    pub const REFERENCE_SAFETY_V3: &'static str = "reference-safety-v3";
    pub const SEQS_IN_BINOPS_CHECK: &'static str = "seqs-in-binops-check";
    pub const SIGNED_INT_REWRITE: &'static str = "signed-int-rewrite";
    pub const SKIP_BAILOUT_ON_EXTENDED_CHECKS: &'static str = "skip-bailout-on-extended-checks";
    pub const SKIP_INLINING_INLINE_FUNS: &'static str = "skip-inlining-inline-funs";
    pub const SPEC_CHECK: &'static str = "spec-check";
    pub const SPEC_REWRITE: &'static str = "spec-rewrite";
    pub const SPLIT_CRITICAL_EDGES: &'static str = "split-critical-edges";
    pub const STOP_AFTER_EXTENDED_CHECKS: &'static str = "stop-after-extended-checks";
    pub const STOP_BEFORE_EXTENDED_CHECKS: &'static str = "stop-before-extended-checks";
    pub const STOP_BEFORE_FILE_FORMAT: &'static str = "stop-before-file-format";
    pub const STOP_BEFORE_STACKLESS_BYTECODE: &'static str = "stop-before-stackless-bytecode";
    pub const UNINITIALIZED_CHECK: &'static str = "uninitialized-check";
    pub const UNSAFE_PACKAGE_VISIBILITY: &'static str = "unsafe-package-visibility";
    pub const UNUSED_ASSIGNMENT_CHECK: &'static str = "unused-assignment-check";
    pub const UNUSED_STRUCT_PARAMS_CHECK: &'static str = "unused-struct-params-check";
    pub const USAGE_CHECK: &'static str = "usage-check";
    pub const VARIABLE_COALESCING: &'static str = "variable-coalescing";
    pub const VARIABLE_COALESCING_ANNOTATE: &'static str = "variable-coalescing-annotate";
}
