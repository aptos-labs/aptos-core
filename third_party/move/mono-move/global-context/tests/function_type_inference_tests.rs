// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for `types::infer_function_type_args`.

use mono_move_core::{
    types::{
        infer_function_type_args, FunctionTypeMismatch, InternedType, ADDRESS_TY, BOOL_TY, U64_TY,
    },
    FunctionSignature, Interner,
};
use mono_move_global_context::GlobalContext;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    ident_str,
};

fn copy_drop() -> AbilitySet {
    AbilitySet::EMPTY | Ability::Copy | Ability::Drop
}

fn signature<I: Interner>(
    interner: &I,
    params: &[InternedType],
    returns: &[InternedType],
) -> FunctionSignature {
    FunctionSignature {
        params: interner.type_list_of(params),
        returns: interner.type_list_of(returns),
    }
}

fn func<I: Interner>(
    interner: &I,
    args: &[InternedType],
    results: &[InternedType],
    abilities: AbilitySet,
) -> InternedType {
    let args = interner.type_list_of(args);
    let results = interner.type_list_of(results);
    interner.function_of(args, results, abilities)
}

/// `0x1::boxes::Box<ty>`.
fn box_of<I: Interner>(interner: &I, ty: InternedType) -> InternedType {
    let module_id = interner.module_id_of(&AccountAddress::ONE, ident_str!("boxes"));
    let name = interner.identifier_of(ident_str!("Box"));
    let ty_args = interner.type_list_of(&[ty]);
    interner.nominal_of(module_id, name, ty_args)
}

#[test]
fn non_generic_signature_infers_nothing() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let declared = signature(&guard, &[U64_TY, U64_TY], &[U64_TY]);
    let expected = func(&guard, &[U64_TY, U64_TY], &[U64_TY], copy_drop());

    assert_eq!(infer_function_type_args(declared, expected, 0), Ok(vec![]));
}

#[test]
fn type_parameters_are_inferred_from_the_expected_type() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let t1 = guard.type_param_of(1);
    let vec_t1 = guard.vector_of(t1);
    let declared = signature(&guard, &[t0, vec_t1], &[t0]);

    let vec_bool = guard.vector_of(BOOL_TY);
    let expected = func(&guard, &[U64_TY, vec_bool], &[U64_TY], copy_drop());

    assert_eq!(
        infer_function_type_args(declared, expected, 2),
        Ok(vec![U64_TY, BOOL_TY])
    );
}

#[test]
fn inference_reaches_into_nominal_type_arguments() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let box_t0 = box_of(&guard, t0);
    let declared = signature(&guard, &[box_t0], &[]);

    let box_address = box_of(&guard, ADDRESS_TY);
    let expected = func(&guard, &[box_address], &[], copy_drop());

    assert_eq!(
        infer_function_type_args(declared, expected, 1),
        Ok(vec![ADDRESS_TY])
    );
}

#[test]
fn a_parameter_used_twice_must_agree() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let declared = signature(&guard, &[t0, t0], &[]);

    let agreeing = func(&guard, &[U64_TY, U64_TY], &[], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, agreeing, 1),
        Ok(vec![U64_TY])
    );

    let conflicting = func(&guard, &[U64_TY, BOOL_TY], &[], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, conflicting, 1),
        Err(FunctionTypeMismatch::Incompatible)
    );
}

#[test]
fn a_reference_is_not_a_valid_type_argument() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let declared = signature(&guard, &[t0], &[]);

    for ref_ty in [guard.immut_ref_of(U64_TY), guard.mut_ref_of(U64_TY)] {
        let expected = func(&guard, &[ref_ty], &[], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, expected, 1),
            Err(FunctionTypeMismatch::Incompatible)
        );
    }
}

#[test]
fn a_reference_parameter_still_matches_a_reference() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let ref_t0 = guard.immut_ref_of(t0);
    let declared = signature(&guard, &[ref_t0], &[]);

    let ref_u64 = guard.immut_ref_of(U64_TY);
    let expected = func(&guard, &[ref_u64], &[], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, expected, 1),
        Ok(vec![U64_TY])
    );

    // Mutability is invariant.
    let mut_u64 = guard.mut_ref_of(U64_TY);
    let expected = func(&guard, &[mut_u64], &[], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, expected, 1),
        Err(FunctionTypeMismatch::Incompatible)
    );
}

#[test]
fn a_nested_function_type_must_have_the_very_same_abilities() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let callback = func(&guard, &[t0], &[], copy_drop());
    let declared = signature(&guard, &[callback], &[]);

    let matching = func(&guard, &[U64_TY], &[], copy_drop());
    let expected = func(&guard, &[matching], &[], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, expected, 1),
        Ok(vec![U64_TY])
    );

    let weaker = func(&guard, &[U64_TY], &[], AbilitySet::EMPTY | Ability::Drop);
    let expected = func(&guard, &[weaker], &[], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, expected, 1),
        Err(FunctionTypeMismatch::Incompatible)
    );
}

#[test]
fn arity_mismatches_are_incompatible() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let declared = signature(&guard, &[U64_TY], &[U64_TY]);

    let too_few_args = func(&guard, &[], &[U64_TY], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, too_few_args, 0),
        Err(FunctionTypeMismatch::Incompatible)
    );

    let too_many_returns = func(&guard, &[U64_TY], &[U64_TY, U64_TY], copy_drop());
    assert_eq!(
        infer_function_type_args(declared, too_many_returns, 0),
        Err(FunctionTypeMismatch::Incompatible)
    );
}

#[test]
fn a_parameter_the_signature_never_mentions_stays_unbound() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let declared = signature(&guard, &[t0], &[]);
    let expected = func(&guard, &[U64_TY], &[], copy_drop());

    assert_eq!(
        infer_function_type_args(declared, expected, 2),
        Err(FunctionTypeMismatch::NotInstantiated)
    );
}

#[test]
fn a_non_function_expected_type_is_rejected() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let declared = signature(&guard, &[], &[]);

    assert_eq!(
        infer_function_type_args(declared, U64_TY, 0),
        Err(FunctionTypeMismatch::NotAFunction)
    );
}
