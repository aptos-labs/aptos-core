// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Compiler V2 transactional-test corpus: Move sources compiled by
//! Compiler V2 and run on the VM, plus Lean sources under `tests/leaner/`.

use crate::{Applicability, Corpus, MatrixConfig, VmBackend};
use move_compiler_v2::Experiment;
use move_model::metadata::LanguageVersion;

/// Settings specific to this corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerV2Payload {
    /// Re-run the test on decompiled-and-recompiled source, comparing against a
    /// config-suffixed baseline.
    pub cross_compile: bool,
}

/// Cross compilation re-runs the compiler, and its baseline records compiler
/// output. V2 is a VM backend, so the extra run would test nothing it owns.
fn effective_payload(payload: &CompilerV2Payload, backend: VmBackend) -> CompilerV2Payload {
    match backend {
        VmBackend::V1 => *payload,
        VmBackend::V2 => CompilerV2Payload {
            cross_compile: false,
        },
    }
}

/// Excluded by every config that takes all tests: these directories are served
/// by the specialized configs below, which need non-default settings.
const COMMON_EXCLUSIONS: &[&str] = &[
    "/leaner/",
    "/operator_eval/",
    "/no-recursive-check/",
    "/no-access-check/",
    "/no-recursive-type-check/",
    "/testing-constant/",
    "/structs_visibility/",
    "/public_const/",
    "/for_loop_lang_2_4/",
];

/// Lean sources need the `lake` toolchain, which V2 CI does not install.
/// They compile to ordinary Move bytecode, so we can change this in the future.
const LEAN_ONLY: Applicability = Applicability::Deferred("requires the lake toolchain");

const CONFIGS: &[MatrixConfig<CompilerV2Payload>] = &[
    // Matches all default experiments.
    MatrixConfig {
        name: "baseline",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &[],
        exclude: COMMON_EXCLUSIONS,
        payload: CompilerV2Payload {
            cross_compile: true,
        },
        v2: Applicability::Applicable,
    },
    // Enables the standard optimization and compare-test passes.
    MatrixConfig {
        name: "optimize",
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
        ],
        language_version: LanguageVersion::latest(),
        include: &[],
        exclude: COMMON_EXCLUSIONS,
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "no-optimize",
        experiments: &[(Experiment::OPTIMIZE, false)],
        language_version: LanguageVersion::latest(),
        include: &[],
        exclude: COMMON_EXCLUSIONS,
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    // Lean-authored programs have their own front end and use one default
    // Compiler V2 configuration outside the generic optimization matrix.
    MatrixConfig {
        name: "leaner",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &["/leaner/"],
        exclude: &[],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: LEAN_ONLY,
    },
    // Let Leaner-permissive borrow programs reach the production bytecode
    // verifier even when compiler-v2's stricter reference checker reports the
    // same program under the dedicated Leaner configuration.
    MatrixConfig {
        name: "no-reference-safety",
        experiments: &[
            (Experiment::REFERENCE_SAFETY_V3, false),
            (Experiment::REFERENCE_SAFETY, false),
        ],
        language_version: LanguageVersion::latest(),
        include: &["/leaner/borrow_checker/leaner_permissive_"],
        exclude: &[],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: LEAN_ONLY,
    },
    // Enables inlining, cross-package inlining, and extra optimizations.
    MatrixConfig {
        name: "opt-extra",
        experiments: &[
            (Experiment::INLINING_OPTIMIZATION, true),
            (Experiment::ACROSS_PACKAGE_INLINING, true),
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_EXTRA, true),
        ],
        language_version: LanguageVersion::latest(),
        include: &[],
        exclude: COMMON_EXCLUSIONS,
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "operator-eval-lang-2",
        experiments: &[(Experiment::OPTIMIZE, true)],
        language_version: LanguageVersion::latest(),
        include: &["/operator_eval/"],
        exclude: &["/structs_visibility/"],
        payload: CompilerV2Payload {
            cross_compile: true,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "no-recursive-check",
        experiments: &[(Experiment::RECURSIVE_TYPE_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &["/no-recursive-check/"],
        exclude: &["/structs_visibility/"],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "no-access-check",
        experiments: &[(Experiment::ACCESS_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &["/no-access-check/"],
        exclude: &["/structs_visibility/"],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "no-recursive-type-check",
        experiments: &[(Experiment::RECURSIVE_TYPE_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &["/no-recursive-type-check/"],
        exclude: &["/structs_visibility/"],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "public-struct",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &["/structs_visibility/"],
        exclude: &[],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "public-const",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &["/public_const/"],
        exclude: &[],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "testing-constant-true",
        experiments: &[(Experiment::COMPILE_FOR_TESTING, true)],
        language_version: LanguageVersion::latest(),
        include: &["/testing-constant/"],
        exclude: &["/structs_visibility/"],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    MatrixConfig {
        name: "testing-constant-false",
        experiments: &[(Experiment::COMPILE_FOR_TESTING, false)],
        language_version: LanguageVersion::latest(),
        include: &["/testing-constant/"],
        exclude: &["/structs_visibility/"],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
    // Under language version 2.4, a `for` loop evaluates its upper bound inside
    // the iterator's scope.
    MatrixConfig {
        name: "for-loop-lang-2.4",
        experiments: &[],
        language_version: LanguageVersion::V2_4,
        include: &["/for_loop_lang_2_4/"],
        exclude: &[],
        payload: CompilerV2Payload {
            cross_compile: false,
        },
        v2: Applicability::Applicable,
    },
];

/// Sources whose result differs by config, so each config gets its own
/// `test.<config>.exp` rather than sharing `test.exp`.
const SEPARATE_BASELINE: &[&str] = &[
    // These run under both the default Leaner config and the config with
    // Compiler V2 reference safety disabled.
    "/leaner/borrow_checker/leaner_permissive_",
    "control_flow/abort_complex.move",
    "control_flow/abort_invalid.move",
    "control_flow/abort_vector.move",
    // Without optimization, this test exceeds local-variable or stack limits.
    "constants/large_vectors.move",
    // Printed bytecode depends on optimization settings.
    "no-v1-comparison/print_bytecode.move",
    "bug_14243_stack_size.move",
    // Output depends on the language version.
    "/operator_eval/",
    // Generated code depends on optimization settings.
    "no-v1-comparison/enum/enum_field_select.move",
    "no-v1-comparison/enum/enum_field_select_different_offsets.move",
    "no-v1-comparison/assert_one.move",
    "no-v1-comparison/closures/reentrancy",
    "no-v1-comparison/structs_visibility/migrated_tests/public_enum_field_select.move",
    "control_flow/for_loop_non_terminating.move",
    "control_flow/for_loop_nested_break.move",
    "evaluation_order/lazy_assert.move",
    "evaluation_order/short_circuiting_invalid.move",
    "evaluation_order/struct_arguments.move",
    "inlining/bug_11223.move",
    "misc/build_with_warnings.move",
    "optimization/bug_14223_unused_non_droppable.move",
    // The redundant-unused-assignment diagnostic is nondeterministic.
    "no-v1-comparison/enum/enum_scoping.move",
    // Diagnostics depend on optimization settings.
    "no-v1-comparison/fv_as_keys.move",
    // Verbose mode includes exact diagnostics.
    "/signed-int/",
    // Expected output depends on whether test-only constants are enabled.
    "/testing-constant/",
];

pub static COMPILER_V2: Corpus<CompilerV2Payload> = Corpus {
    name: "compiler-v2",
    root: "third_party/move/move-compiler-v2/transactional-tests",
    source_extensions: &["move", "lean"],
    configs: CONFIGS,
    separate_baseline: SEPARATE_BASELINE,
    effective_payload,
};
