// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Move VM transactional-test corpus.

use crate::{Applicability, Corpus, MatrixConfig};
use move_bytecode_verifier::{verifier::VerificationScope, VerifierConfig};
use move_compiler_v2::Experiment;
use move_model::metadata::LanguageVersion;
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

/// Production verifier.
const PRODUCTION: VMConfig = vm_config(VerifierConfig::production());

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

/// Paranoid modes run with the verifier disabled so that unverified bytecode
/// reaches the interpreter's own type checks. V2 always runs verification
/// before specializing and has no paranoid mode, so these configs describe
/// machinery it does not have.
const NO_PARANOID_MODE: Applicability =
    Applicability::NotApplicable("V2 always verifies and has no paranoid check mode");

const CONFIGS: &[MatrixConfig<MoveVmPayload>] = &[
    MatrixConfig {
        name: "baseline",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &[],
        exclude: &["/paranoid-tests/", "/tracing/"],
        payload: MoveVmPayload {
            vm_config: PRODUCTION,
            tracing: false,
        },
        v2: Applicability::Applicable,
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
        v2: NO_PARANOID_MODE,
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
        v2: NO_PARANOID_MODE,
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
                ..PRODUCTION
            },
            tracing: false,
        },
        v2: Applicability::Deferred("V2 has its own loading policies; the mapping is undecided"),
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
        v2: NO_PARANOID_MODE,
    },
    MatrixConfig {
        name: "tracing",
        experiments: &[],
        language_version: LanguageVersion::latest(),
        include: &["/tracing/"],
        exclude: &[],
        payload: MoveVmPayload {
            vm_config: PRODUCTION,
            tracing: true,
        },
        v2: Applicability::NotApplicable("replays V1's own type checks from a collected trace"),
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

pub static MOVE_VM: Corpus<MoveVmPayload> = Corpus {
    name: "move-vm",
    root: "third_party/move/move-vm/transactional-tests",
    source_extensions: &["move", "masm"],
    configs: CONFIGS,
    separate_baseline: SEPARATE_BASELINE,
    // Applicable configs run the same payload on V1 and V2.
    effective_payload: |payload, _backend| payload.clone(),
};
