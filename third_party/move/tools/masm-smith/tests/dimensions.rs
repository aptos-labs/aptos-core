// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks each `valid_*` flag: with the flag off, generated tests still assemble, the verifier
//! rejects some of them, and every rejection comes from that rule's own check.

use arbitrary::Unstructured;
use either::Either;
use masm_smith::{
    assembler_options, generate, split_assembler_sections, stdlib_modules, Config, DimensionFlag,
};
use move_asm::assembler;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::{verifier, VerifierConfig};
use move_core_types::vm_status::StatusCode;
use std::collections::BTreeSet;

const SWEEP_ITERATIONS: u32 = 96;
const SEED_BYTES: usize = 192;

fn seed_for(index: u32) -> Vec<u8> {
    masm_smith::seeded_bytes(
        u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xC6BC_2796_92B5_C323,
        SEED_BYTES,
    )
}

/// Run a sweep with the given config and return `(at_least_one_assembled, at_least_one_rejected)`.
/// Earlier publishes within an iteration are fed as context for later publishes, mirroring
/// the transactional-test runner.
///
/// A config that violates limits is verified with `VerifierConfig::production_testing()`: the
/// default config leaves most numeric caps unset, so a limits violation would never trip.
fn sweep(cfg: &Config) -> (bool, bool) {
    sweep_with_iters(cfg, SWEEP_ITERATIONS)
}

fn sweep_with_iters(cfg: &Config, iters: u32) -> (bool, bool) {
    let (assembled, statuses) = sweep_statuses(cfg, iters);
    (assembled, !statuses.is_empty())
}

/// Like [`sweep_with_iters`], but returns the set of verifier rejection statuses.
fn sweep_statuses(cfg: &Config, iters: u32) -> (bool, BTreeSet<StatusCode>) {
    let verifier_config = if !cfg.valid_limits {
        VerifierConfig::production_testing()
    } else {
        VerifierConfig::default()
    };
    let mut assembled = false;
    let mut statuses = BTreeSet::new();
    for i in 0..iters {
        let seed = seed_for(i);
        let mut u = Unstructured::new(&seed);
        let Ok(test) = generate(&mut u, cfg) else {
            continue;
        };
        let text = test.render();
        let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
        for (section_idx, section) in split_assembler_sections(&text).iter().enumerate() {
            let opts = assembler_options();
            match assembler::assemble(&opts, section, context.iter()) {
                Ok(Either::Left(module)) => {
                    assembled = true;
                    // Verify a corrupted clone so the uncorrupted version stays valid as
                    // context for later sections — some post-process mutations (`valid_bounds`
                    // rewrites an out-of-range table index) leave the module unsafe to read
                    // and would panic the assembler when reused.
                    let mut for_verify = module.clone();
                    test.post_process_module(section_idx, &mut for_verify);
                    if let Err(err) =
                        verifier::verify_module_with_config(&verifier_config, &for_verify)
                    {
                        statuses.insert(err.major_status());
                    }
                    context.push(module);
                },
                Ok(Either::Right(script)) => {
                    assembled = true;
                    if let Err(err) = verifier::verify_script_with_config(&verifier_config, &script)
                    {
                        statuses.insert(err.major_status());
                    }
                },
                Err(_) => {
                    // Under `valid_*` flags set to `false`, the assembler must still accept
                    // the output — structural correctness is unconditional. Any assembly
                    // failure is a generator bug.
                    panic!(
                        "dimensions sweep: assembly failed under flagged-off config\n---section---\n{}",
                        section
                    );
                },
            }
        }
    }
    (assembled, statuses)
}

#[test]
fn abilities_violation_eventually_rejects() {
    let cfg = Config {
        valid_abilities: false,
        enable_variants: false,
        enable_generics: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_abilities=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn struct_api_violation_eventually_rejects() {
    let cfg = Config {
        valid_struct_api: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, statuses) = sweep_statuses(&cfg, SWEEP_ITERATIONS);
    assert!(assembled, "no module assembled in the sweep");
    // Every rejection must come from the struct API checker, not an earlier pass.
    assert!(
        statuses == BTreeSet::from([StatusCode::INVALID_STRUCT_API_CODE]),
        "expected only INVALID_STRUCT_API_CODE rejections across {} seeds with \
         valid_struct_api=false, got {:?}",
        SWEEP_ITERATIONS,
        statuses
    );
}

#[test]
fn acquires_violation_eventually_rejects() {
    let cfg = Config {
        valid_acquires: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_acquires=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn friends_violation_eventually_rejects() {
    let cfg = Config {
        valid_friends: false,
        violation_injection_prob: 200,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_friends=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn constants_violation_eventually_rejects() {
    let cfg = Config {
        valid_constants: false,
        violation_injection_prob: 200,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_constants=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn instantiation_loops_violation_eventually_rejects() {
    let cfg = Config {
        valid_instantiation_loops: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_instantiation_loops=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn struct_defs_violation_eventually_rejects() {
    let cfg = Config {
        valid_struct_defs: false,
        violation_injection_prob: 128,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_struct_defs=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn reference_safety_violation_eventually_rejects() {
    // Restrict competing features and use frequent injection so the seed sweep reaches
    // `try_mut_borrow_loc` while a reference-safety violation is possible.
    let cfg = Config {
        valid_reference_safety: false,
        violation_injection_prob: 255,
        enable_generics: false,
        enable_closures: false,
        enable_struct_api: false,
        ..Config::default()
    };
    const ITERS: u32 = 512;
    let (assembled, rejected) = sweep_with_iters(&cfg, ITERS);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_reference_safety=false",
        ITERS
    );
}

#[test]
fn duplication_violation_eventually_rejects() {
    let cfg = Config {
        valid_duplication: false,
        violation_injection_prob: 128,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_duplication=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn type_safety_violation_eventually_rejects() {
    let cfg = Config {
        valid_type_safety: false,
        violation_injection_prob: 128,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_type_safety=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn locals_safety_violation_eventually_rejects() {
    let cfg = Config {
        valid_locals_safety: false,
        violation_injection_prob: 128,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_locals_safety=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn limits_violation_eventually_rejects() {
    // The limits violation generates an oversized struct (35 fields > the production
    // `max_fields_in_struct = 30`). Default `VerifierConfig` doesn't set this cap, so the
    // test must opt into production limits to surface the rejection.
    let cfg = Config {
        valid_limits: false,
        // Favor singleton structs so the sweep exercises the field-count violation.
        enable_variants: false,
        enable_generics: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_limits=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn signature_v2_violation_eventually_rejects() {
    // `try_pack` needs a reference at a generic field's stack position to infer a type
    // argument that violates its constraints, such as `&u64` for `T: store`. Keep references
    // and generics enabled; disabling signers preserves the seed choices that reach this case.
    let cfg = Config {
        valid_signature_v2: false,
        violation_injection_prob: 255,
        enable_signers: false,
        ..Config::default()
    };
    // Use a larger sweep because references rarely occupy the needed generic-field position.
    const ITERS: u32 = 512;
    let (assembled, rejected) = sweep_with_iters(&cfg, ITERS);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_signature_v2=false",
        ITERS
    );
}

#[test]
fn bounds_violation_eventually_rejects() {
    // The bounds violation rewrites the first function-def's `FunctionHandleIndex` to one
    // past the end of the function-handle table at post-process time. `try_call` is the host
    // generator — any successful call invocation can flip the module-wide flag, and the
    // verifier's `check_bounds` pass walks every table reference at module load.
    let cfg = Config {
        valid_bounds: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_bounds=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn instruction_consistency_violation_eventually_rejects() {
    // The violation rewrites the first `VecPack` instruction's `num` operand past
    // `u16::MAX` at post-process time. The `instruction_consistency` pass rejects with
    // `CONSTRAINT_NOT_SATISFIED`. `try_vec_pack` is the host generator — the corruption
    // can only fire when at least one `VecPack` lands in the body.
    let cfg = Config {
        valid_instruction_consistency: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_instruction_consistency=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn control_flow_violation_eventually_rejects() {
    // The control-flow violation emits a four-block irreducible CFG (block 0 conditionally
    // jumps to block 2 or falls through to block 1; block 1 unconditionally branches to
    // block 2; block 2 conditionally back-edges to block 1 — but block 1 doesn't dominate
    // block 2, so the cycle isn't a natural loop). `control_flow.rs` rejects via the
    // reducibility check. The shape needs `block_count >= 4`, which the default config
    // (`blocks_per_fun_max = 8`) provides comfortably.
    let cfg = Config {
        valid_control_flow: false,
        violation_injection_prob: 255,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        rejected,
        "no verifier rejection across {} seeds with valid_control_flow=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn stack_balance_violation_eventually_rejects() {
    // Bump the injection probability a bit so a small sweep is enough to surface rejections.
    let cfg = Config {
        valid_stack_balance: false,
        violation_injection_prob: 96,
        ..Config::default()
    };
    let (assembled, rejected) = sweep(&cfg);
    assert!(
        assembled,
        "no module assembled in the sweep — generator never produced anything"
    );
    assert!(
        rejected,
        "no verifier rejection observed across {} seeds with valid_stack_balance=false",
        SWEEP_ITERATIONS
    );
}

#[test]
fn struct_api_wrappers_verify_under_default_config() {
    // Valid wrappers are emitted under the default config; the struct API checker must accept
    // every one of them at the newest bytecode version.
    let (assembled, statuses) = sweep_statuses(&Config::default(), SWEEP_ITERATIONS);
    assert!(assembled, "no module assembled in the sweep");
    assert!(
        statuses.is_empty(),
        "default-config modules rejected: {:?}",
        statuses
    );
}

/// Status codes the verifier pass targeted by each dimension may report. A rejection from
/// any other pass means the injection reached the wrong check.
fn expected_statuses(dim: DimensionFlag) -> &'static [StatusCode] {
    use StatusCode::*;
    match dim {
        DimensionFlag::TypeSafety => &[STLOC_TYPE_MISMATCH_ERROR],
        DimensionFlag::LocalsSafety => &[MOVELOC_UNAVAILABLE_ERROR],
        DimensionFlag::ReferenceSafety => &[COPYLOC_EXISTS_BORROW_ERROR],
        DimensionFlag::Abilities => &[FIELD_MISSING_TYPE_ABILITY],
        DimensionFlag::ControlFlow => &[INVALID_LOOP_SPLIT],
        DimensionFlag::StackBalance => &[
            NEGATIVE_STACK_SIZE_WITHIN_BLOCK,
            POSITIVE_STACK_SIZE_AT_BLOCK_END,
            RET_TYPE_MISMATCH_ERROR,
        ],
        DimensionFlag::Acquires => &[MISSING_ACQUIRES_ANNOTATION],
        DimensionFlag::Bounds => &[INDEX_OUT_OF_BOUNDS],
        DimensionFlag::Duplication => &[DUPLICATE_ELEMENT],
        DimensionFlag::SignatureV2 => &[CONSTRAINT_NOT_SATISFIED, INVALID_SIGNATURE_TOKEN],
        DimensionFlag::InstantiationLoops => &[LOOP_IN_INSTANTIATION_GRAPH],
        DimensionFlag::Constants => &[MALFORMED_CONSTANT_DATA],
        DimensionFlag::StructDefs => &[RECURSIVE_STRUCT_DEFINITION],
        DimensionFlag::Friends => &[INVALID_FRIEND_DECL_WITH_SELF],
        DimensionFlag::Limits => &[MAX_FIELD_DEFINITIONS_REACHED],
        DimensionFlag::StructApi => &[INVALID_STRUCT_API_CODE],
        DimensionFlag::InstructionConsistency => &[CONSTRAINT_NOT_SATISFIED],
    }
}

#[test]
fn every_dimension_rejects_only_in_its_own_pass() {
    for &dim in DimensionFlag::ALL {
        let mut cfg = Config {
            violation_injection_prob: 255,
            ..Config::default()
        };
        cfg.set_flag(dim, false);
        let (_, statuses) = sweep_statuses(&cfg, 512);
        let expected: BTreeSet<StatusCode> = expected_statuses(dim).iter().copied().collect();
        let unexpected: BTreeSet<&StatusCode> = statuses.difference(&expected).collect();
        assert!(
            unexpected.is_empty(),
            "{}: rejected by an unexpected pass with {:?}",
            dim.as_kebab(),
            unexpected
        );
    }
}
