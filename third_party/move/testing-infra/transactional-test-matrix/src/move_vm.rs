// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Move VM transactional-test corpus.

use crate::{Applicability, Corpus, MatrixConfig, MonoMoveDivergence, Resolution};
use move_bytecode_verifier::{verifier::VerificationScope, VerifierConfig};
use move_compiler_v2::Experiment;
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::vm_test_harness::TestRunConfig;
use move_vm_runtime::config::VMConfig;

/// Shared VM configuration for the Move VM transactional tests.
const fn vm_config(verifier_config: VerifierConfig) -> VMConfig {
    VMConfig {
        paranoid_type_checks: true,
        optimize_trusted_code: true,
        verifier_config,
        enable_enum_option: false,
        ..VMConfig::default_for_test()
    }
}

/// Approximates the production verifier config.
const PRODUCTION_TESTING: VMConfig = vm_config(VerifierConfig::production_testing());

/// Verifier disabled.
const VERIFIER_OFF: VMConfig =
    vm_config(VerifierConfig::unbounded().set_scope(VerificationScope::Nothing));

/// Settings specific to this corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveVmPayload {
    pub vm_config: VMConfig,
    /// Delay runtime type checks to a post-execution replay of the collected
    /// trace, instead of checking in place.
    pub tracing: bool,
}

/// Builds runner settings with the resolved VM config and tracing mode.
fn test_run_config(resolution: &Resolution<'_, MoveVmPayload>) -> TestRunConfig {
    TestRunConfig {
        vm_config: resolution.effective_payload.vm_config.clone(),
        tracing: resolution.effective_payload.tracing,
        ..resolution.config.run_config()
    }
}

/// Paranoid modes run with the verifier disabled so that unverified bytecode
/// reaches the interpreter's own type checks. MonoMove always runs verification
/// before specializing and has no paranoid mode, so these configs describe
/// machinery it does not have.
const NO_PARANOID_MODE: Applicability =
    Applicability::NotApplicable("MonoMove always verifies and has no paranoid check mode");

const CONFIGS: &[MatrixConfig<MoveVmPayload>] = &[
    MatrixConfig {
        name: "baseline",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &[],
        exclude: &["/paranoid-tests/", "/tracing/"],
        payload: MoveVmPayload {
            vm_config: PRODUCTION_TESTING,
            tracing: false,
        },
        mono_move: Applicability::Applicable,
    },
    MatrixConfig {
        name: "async-paranoid",
        // For function-value tests, deferred checks omit the stack traces that
        // in-place checks attach.
        experiments: &[(Experiment::ACCESS_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &[
            "/limits/",
            "/function_values_safety/",
            "/paranoid-tests/",
            "/stack_size/",
            "/trusted_code/",
        ],
        exclude: &[],
        payload: MoveVmPayload {
            vm_config: VERIFIER_OFF,
            tracing: true,
        },
        mono_move: NO_PARANOID_MODE,
    },
    MatrixConfig {
        name: "paranoid",
        experiments: &[(Experiment::ACCESS_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &[
            "/limits/",
            "/function_values_safety/",
            "/paranoid-tests/",
            "/stack_size/",
            "/trusted_code/",
        ],
        exclude: &[],
        payload: MoveVmPayload {
            vm_config: VERIFIER_OFF,
            tracing: false,
        },
        mono_move: NO_PARANOID_MODE,
    },
    MatrixConfig {
        name: "eager-loading",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &[],
        exclude: &[
            "/function_values_safety/",
            "/lazy_loading/",
            "/limits/",
            "/paranoid-tests/",
            "/runtime_ref_checks/",
            "/stack_size/",
            "/tracing/",
            "/trusted_code/",
            "/struct_api/",
        ],
        payload: MoveVmPayload {
            vm_config: VMConfig {
                enable_lazy_loading: false,
                ..PRODUCTION_TESTING
            },
            tracing: false,
        },
        mono_move: Applicability::Deferred(
            "MonoMove has its own loading policies; the mapping is undecided",
        ),
    },
    // Runs the runtime reference-checker corpus.
    MatrixConfig {
        name: "ref",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &["/runtime_ref_checks/"],
        exclude: &[],
        payload: MoveVmPayload {
            vm_config: VERIFIER_OFF.set_paranoid_ref_checks(true),
            tracing: false,
        },
        mono_move: NO_PARANOID_MODE,
    },
    MatrixConfig {
        name: "tracing",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &["/tracing/"],
        exclude: &[],
        payload: MoveVmPayload {
            vm_config: PRODUCTION_TESTING,
            tracing: true,
        },
        mono_move: Applicability::NotApplicable(
            "replays V1's own type checks from a collected trace",
        ),
    },
];

/// Sources whose result differs by config, so each config gets its own
/// `test.<config>.exp` rather than sharing `test.exp`.
const SEPARATE_BASELINE: &[&str] = &[
    "/function_values_safety/",
    "/limits/",
    "/module_publishing/",
    "/re_entrancy/",
    "/runtime_ref_checks/",
    "/stack_size/",
    "/trusted_code/",
];

/// Sources whose MonoMove output differs from the canonical baseline.
const MONO_MOVE_DIVERGENCES: &[MonoMoveDivergence] = &[
    MonoMoveDivergence::rendering(
        "tests/builtins/get_missing_struct.masm",
        "no stack trace in exec_state",
    ),
    MonoMoveDivergence::unsupported(
        "tests/display/print_values.move",
        "returned enums have no layout, function values",
    ),
    MonoMoveDivergence::unsupported(
        "tests/entry_points/generic_return_values.move",
        "multiple return values",
    ),
    MonoMoveDivergence::unsupported(
        "tests/entry_points/modify_mutable_ref_inputs.masm",
        "reference parameters",
    ),
    MonoMoveDivergence::unsupported("tests/entry_points/ref_inputs.masm", "reference parameters"),
    MonoMoveDivergence::unsupported(
        "tests/entry_points/return_values.masm",
        "reference parameters",
    ),
    MonoMoveDivergence::unsupported(
        "tests/entry_points/script_too_few_type_args.masm",
        "no entry type-argument validation: a count mismatch is reported as unsupported",
    ),
    MonoMoveDivergence::semantic(
        "tests/entry_points/script_too_few_type_args_inner.masm",
        "no entry type-argument validation: a struct instantiated with the wrong arity runs",
    ),
    MonoMoveDivergence::unsupported(
        "tests/entry_points/script_too_many_type_args.masm",
        "no entry type-argument validation: a count mismatch is reported as unsupported",
    ),
    MonoMoveDivergence::semantic(
        "tests/entry_points/script_too_many_type_args_inner.masm",
        "no entry type-argument validation: a struct instantiated with the wrong arity runs",
    ),
    MonoMoveDivergence::semantic(
        "tests/entry_points/script_type_arg_kind_mismatch_1.masm",
        "no entry type-argument validation: an ability constraint violation runs",
    ),
    MonoMoveDivergence::semantic(
        "tests/entry_points/script_type_arg_kind_mismatch_2.masm",
        "no entry type-argument validation: an ability constraint violation runs",
    ),
    MonoMoveDivergence::unsupported(
        "tests/entry_points/struct_arguments.masm",
        "reference parameters",
    ),
    MonoMoveDivergence::unsupported(
        "tests/function_values_safety/closure_assign_return.masm",
        "multiple return values",
    ),
    MonoMoveDivergence::rendering(
        "tests/function_values_safety/dep_compatibility.masm",
        "invariant violation message, sub-status, and location for a stale closure",
    ),
    MonoMoveDivergence::unsupported(
        "tests/instructions/closure_equality.masm",
        "function value equality",
    ),
    MonoMoveDivergence::semantic(
        "tests/lazy_loading/cyclic_structs.masm",
        "no value depth limit; cyclic struct types are unsupported",
    ),
    MonoMoveDivergence::semantic(
        "tests/limits/borrow_global_type_too_large.masm",
        "no type depth limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/limits/local_type_too_large.masm",
        "no type depth limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/limits/pack_closure_generic_type_too_large.masm",
        "no type size limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/limits/pack_closure_ret_type_too_deep.masm",
        "no type depth limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/limits/pack_generic_type_too_large.masm",
        "no type depth limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/limits/pack_variant_generic_type_too_large.masm",
        "no type depth limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/limits/vec_pack_type_too_large.masm",
        "no type depth limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/re_entrancy/cyclic_calls.masm",
        "no reentrancy checks",
    ),
    MonoMoveDivergence::semantic(
        "tests/re_entrancy/cyclic_closure_calls.masm",
        "no reentrancy checks",
    ),
    MonoMoveDivergence::semantic(
        "tests/recursion/runtime_layout_deeply_nested.masm",
        "no value depth limit",
    ),
    MonoMoveDivergence::semantic(
        "tests/recursion/runtime_type_deeply_nested.masm",
        "no type depth limit",
    ),
    MonoMoveDivergence::rendering(
        "tests/stack_size/unpack_variant_generic_mismatch.masm",
        "variant mismatch message names no variant",
    ),
    MonoMoveDivergence::rendering(
        "tests/stack_size/unpack_variant_mismatch.masm",
        "variant mismatch message names no variant",
    ),
];

pub static MOVE_VM: Corpus<MoveVmPayload> = Corpus {
    name: "move-vm",
    root: "third_party/move/move-vm/transactional-tests",
    source_extensions: &["move", "masm"],
    configs: CONFIGS,
    separate_baseline: SEPARATE_BASELINE,
    // Applicable configs run the same payload on V1 and MonoMove.
    effective_payload: |payload, _backend| payload.clone(),
    test_run_config,
    mono_move_divergences: MONO_MOVE_DIVERGENCES,
};
