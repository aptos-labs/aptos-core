// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for `Interner::subst_type` and `Interner::subst_type_list`.

use mono_move_core::{
    types::{
        view_type, view_type_list, Type, ADDRESS_TY, BOOL_TY, EMPTY_TYPE_LIST, U128_TY, U64_TY,
    },
    Interner,
};
use mono_move_global_context::GlobalContext;
use move_core_types::{ability::AbilitySet, account_address::AccountAddress, ident_str};

#[test]
fn nested_vector_type_param() {
    // `vector<vector<TypeParam(0)>>` with subst [u64] -> `vector<vector<u64>>`.
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let v_t0 = guard.vector_of(t0);
    let vv_t0 = guard.vector_of(v_t0);

    let ty_args = guard.type_list_of(&[U64_TY]);
    let out = guard.subst_type(vv_t0, ty_args).unwrap();

    // Expected canonical pointer, built independently.
    let v_u64 = guard.vector_of(U64_TY);
    let vv_u64 = guard.vector_of(v_u64);
    assert!(
        out == vv_u64,
        "substituted vector<vector<T0>> must canonicalize to vector<vector<u64>>"
    );
}

#[test]
fn references_substitute() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let r_t0 = guard.immut_ref_of(t0);
    let m_t0 = guard.mut_ref_of(t0);

    let ty_args = guard.type_list_of(&[BOOL_TY]);
    let out_r = guard.subst_type(r_t0, ty_args).unwrap();
    let out_m = guard.subst_type(m_t0, ty_args).unwrap();

    let expected_r = guard.immut_ref_of(BOOL_TY);
    let expected_m = guard.mut_ref_of(BOOL_TY);
    assert!(out_r == expected_r);
    assert!(out_m == expected_m);
}

#[test]
fn function_substitutes_args_and_results() {
    // `fn(T0, u64) -> T0` with subst [address] -> `fn(address, u64) -> address`.
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let args = guard.type_list_of(&[t0, U64_TY]);
    let results = guard.type_list_of(&[t0]);
    let f = guard.function_of(args, results, AbilitySet::EMPTY);

    let ty_args = guard.type_list_of(&[ADDRESS_TY]);
    let out = guard.subst_type(f, ty_args).unwrap();

    let expected_args = guard.type_list_of(&[ADDRESS_TY, U64_TY]);
    let expected_results = guard.type_list_of(&[ADDRESS_TY]);
    let expected = guard.function_of(expected_args, expected_results, AbilitySet::EMPTY);
    assert!(out == expected);
}

#[test]
fn nominal_substitutes_ty_args() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));
    let name = guard.identifier_of(ident_str!("S"));

    let t0 = guard.type_param_of(0);
    let generic_ty_args = guard.type_list_of(&[t0]);
    let generic = guard.nominal_of(module_id, name, generic_ty_args);

    let ty_args = guard.type_list_of(&[U128_TY]);
    let out = guard.subst_type(generic, ty_args).unwrap();

    let expected_ty_args = guard.type_list_of(&[U128_TY]);
    let expected = guard.nominal_of(module_id, name, expected_ty_args);
    assert!(
        out == expected,
        "Nominal must re-canonicalize against its substituted ty_args"
    );

    // The substituted nominal must structurally be a Nominal with the same
    // executable_id / name.
    match view_type(out) {
        Type::Nominal {
            module_id: out_eid,
            name: out_name,
            ty_args: out_args,
            ..
        } => {
            assert!(*out_eid == module_id);
            assert!(*out_name == name);
            let args_slice = view_type_list(*out_args);
            assert_eq!(args_slice.len(), 1);
            assert!(args_slice[0] == U128_TY);
        },
        _ => panic!("expected Nominal after substitution"),
    }
}

#[test]
fn recanonicalization_two_inputs_same_output() {
    // Two different inputs that substitute to the same concrete type must
    // produce pointer-equal outputs. This is the core canonicalization
    // guarantee.
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    // Input 1: `vector<TypeParam(0)>` with subst [u64].
    let t0 = guard.type_param_of(0);
    let v_t0 = guard.vector_of(t0);
    let subst1 = guard.type_list_of(&[U64_TY]);
    let out1 = guard.subst_type(v_t0, subst1).unwrap();

    // Input 2: `vector<TypeParam(1)>` with subst [bool, u64]. Different
    // surface form, same concrete type after substitution.
    let t1 = guard.type_param_of(1);
    let v_t1 = guard.vector_of(t1);
    let subst2 = guard.type_list_of(&[BOOL_TY, U64_TY]);
    let out2 = guard.subst_type(v_t1, subst2).unwrap();

    assert!(out1 == out2);
    let expected = guard.vector_of(U64_TY);
    assert!(out1 == expected);
}

#[test]
fn out_of_bounds_type_param_errors() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t5 = guard.type_param_of(5);
    let ty_args = guard.type_list_of(&[U64_TY]);

    let res = guard.subst_type(t5, ty_args);
    assert!(res.is_err());
}

#[test]
fn subst_type_list_substitutes_each_element() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let t1 = guard.type_param_of(1);
    let list = guard.type_list_of(&[t0, U64_TY, t1]);

    let ty_args = guard.type_list_of(&[ADDRESS_TY, BOOL_TY]);
    let out = guard.subst_type_list(list, ty_args).unwrap();

    let expected = guard.type_list_of(&[ADDRESS_TY, U64_TY, BOOL_TY]);
    assert!(out == expected);
}

#[test]
fn subst_type_list_fast_path_returns_input_pointer() {
    // No element in the list mentions any `TypeParam`, so the list pointer
    // must be returned unchanged (no allocation, no map probe).
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let list = guard.type_list_of(&[U64_TY, BOOL_TY, ADDRESS_TY]);
    let out = guard.subst_type_list(list, EMPTY_TYPE_LIST).unwrap();
    assert!(out == list);
}

#[test]
fn failed_subst_preserves_interner_consistency() {
    // Build `fn(vector<T0>, T5) -> ()`. With ty_args [U64], the first arg
    // would canonicalize to `vector<U64>` (interned as a side effect) and
    // the second arg fails with OOB. The interner must remain consistent.
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let t0 = guard.type_param_of(0);
    let t5 = guard.type_param_of(5);
    let v_t0 = guard.vector_of(t0);
    let args = guard.type_list_of(&[v_t0, t5]);
    let f = guard.function_of(args, EMPTY_TYPE_LIST, AbilitySet::EMPTY);

    let ty_args = guard.type_list_of(&[U64_TY]);
    assert!(guard.subst_type(f, ty_args).is_err());

    // Canonicalization still holds: the same `vector<u64>` pointer is
    // returned regardless of how it is reached.
    let v_u64_direct = guard.vector_of(U64_TY);
    let v_u64_via_subst = guard.subst_type(v_t0, ty_args).unwrap();
    assert!(v_u64_direct == v_u64_via_subst);

    // Repeating the failing call still fails the same way.
    assert!(guard.subst_type(f, ty_args).is_err());
}
