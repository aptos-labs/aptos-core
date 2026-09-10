// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for `is_assignable` on interned types.

use mono_move_core::{
    intern_sig_token, is_assignable,
    types::{BOOL_TY, EMPTY_TYPE_LIST, U64_TY},
    Interner, PreparedModule,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_testsuite::with_loaded_module;
use move_binary_format::file_format::SignatureToken;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    ident_str,
};

/// A trivial module, used only to satisfy `intern_sig_token`'s signature. Every
/// token generated below is struct-free, so the module is never consulted.
const HOST_MODULE: &str = r#"
module 0x1::a {
    struct S has copy, drop, store { x: u64 }
    public fun f(_s: &S) {}
}
"#;

fn with_context(check: impl FnOnce(&ExecutionGuard, &PreparedModule)) {
    with_loaded_module(HOST_MODULE, ident_str!("a"), |guard, module_ir| {
        check(guard, &module_ir.module)
    })
    .expect("module load failed");
}

fn func(
    args: Vec<SignatureToken>,
    results: Vec<SignatureToken>,
    abilities: &[Ability],
) -> SignatureToken {
    let abilities = abilities
        .iter()
        .fold(AbilitySet::EMPTY, |acc, ability| acc | *ability);
    SignatureToken::Function(args, results, abilities)
}

/// Struct-free tokens (interning a struct token needs a module) covering every
/// constructor, plus the shapes where disagreement is plausible: differing
/// abilities and arities, references over function types, and nesting.
///
/// Enumerate rather than generate: generators omit function types, without
/// which every pair reduces to equality.
fn token_pool() -> Vec<SignatureToken> {
    use SignatureToken::*;

    let copy_drop = &[Ability::Copy, Ability::Drop][..];
    let drop_only = &[Ability::Drop][..];
    let all = &[Ability::Copy, Ability::Drop, Ability::Store][..];

    let f_none = func(vec![U64], vec![U64], &[]);
    let f_drop = func(vec![U64], vec![U64], drop_only);
    let f_copy_drop = func(vec![U64], vec![U64], copy_drop);
    let f_all = func(vec![U64], vec![U64], all);
    // Same abilities, different shapes.
    let f_other_arg = func(vec![Bool], vec![U64], copy_drop);
    let f_other_ret = func(vec![U64], vec![Bool], copy_drop);
    let f_extra_arg = func(vec![U64, U64], vec![U64], copy_drop);
    let f_no_ret = func(vec![U64], vec![], copy_drop);
    // Function types nested inside a function's own signature.
    let f_nested_arg = func(vec![f_drop.clone()], vec![], copy_drop);
    let f_nested_arg_strong = func(vec![f_copy_drop.clone()], vec![], copy_drop);

    vec![
        Bool,
        U8,
        U64,
        U128,
        I64,
        I128,
        Address,
        Signer,
        TypeParameter(0),
        TypeParameter(1),
        Vector(Box::new(U64)),
        Vector(Box::new(Bool)),
        Vector(Box::new(f_drop.clone())),
        Vector(Box::new(f_copy_drop.clone())),
        Reference(Box::new(U64)),
        Reference(Box::new(f_none.clone())),
        Reference(Box::new(f_drop.clone())),
        Reference(Box::new(f_copy_drop.clone())),
        Reference(Box::new(Reference(Box::new(f_drop.clone())))),
        Reference(Box::new(Reference(Box::new(f_copy_drop.clone())))),
        MutableReference(Box::new(U64)),
        MutableReference(Box::new(f_none.clone())),
        MutableReference(Box::new(f_drop.clone())),
        MutableReference(Box::new(f_copy_drop.clone())),
        f_none,
        f_drop,
        f_copy_drop,
        f_all,
        f_other_arg,
        f_other_ret,
        f_extra_arg,
        f_no_ret,
        f_nested_arg,
        f_nested_arg_strong,
    ]
}

/// Every ordered pair from the pool must get the same answer from the derived
/// rule and from `is_assignable_from`.
#[test]
fn agrees_with_the_verifier_on_all_pairs() {
    with_context(|guard, module| {
        let pool = token_pool();
        let interned = pool
            .iter()
            .map(|tok| intern_sig_token(tok, module, guard))
            .collect::<Vec<_>>();

        let mut assignable_unequal = 0;
        for (i, expected_tok) in pool.iter().enumerate() {
            for (j, actual_tok) in pool.iter().enumerate() {
                let oracle = expected_tok.is_assignable_from(actual_tok);
                let derived = is_assignable(interned[i], interned[j]);
                assert_eq!(
                    oracle, derived,
                    "disagreement on expected={expected_tok:?}, actual={actual_tok:?}"
                );
                if i != j && derived {
                    assignable_unequal += 1;
                }
            }
        }
        // Guard against a vacuous sweep: the pool must contain pairs that are
        // assignable without being identical, i.e. the ability-widening arm.
        assert!(
            assignable_unequal > 0,
            "pool exercises no non-trivial assignability"
        );
    });
}

#[test]
fn function_abilities_widen_but_do_not_narrow() {
    with_context(|guard, _module| {
        let args = guard.type_list_of(&[U64_TY]);
        let weak = guard.function_of(args, EMPTY_TYPE_LIST, AbilitySet::EMPTY);
        let strong = guard.function_of(args, EMPTY_TYPE_LIST, AbilitySet::PUBLIC_FUNCTIONS);

        // A context wanting fewer abilities accepts a value that has more.
        assert!(is_assignable(weak, strong));
        // The reverse is not sound.
        assert!(!is_assignable(strong, weak));
    });
}

#[test]
fn immutable_references_recurse_but_mutable_ones_do_not() {
    with_context(|guard, _module| {
        let args = guard.type_list_of(&[U64_TY]);
        let weak = guard.function_of(args, EMPTY_TYPE_LIST, AbilitySet::EMPTY);
        let strong = guard.function_of(args, EMPTY_TYPE_LIST, AbilitySet::PUBLIC_FUNCTIONS);

        // Covariant: reading through `&` yields a value usable as `weak`.
        assert!(is_assignable(
            guard.immut_ref_of(weak),
            guard.immut_ref_of(strong)
        ));
        assert!(!is_assignable(
            guard.immut_ref_of(strong),
            guard.immut_ref_of(weak)
        ));

        // Invariant: writing through `&mut` could install a weaker value that
        // the original holder would then use at the stronger type.
        assert!(!is_assignable(
            guard.mut_ref_of(weak),
            guard.mut_ref_of(strong)
        ));
        assert!(!is_assignable(
            guard.mut_ref_of(strong),
            guard.mut_ref_of(weak)
        ));

        // No implicit freeze: `&mut T` is not usable where `&T` is required,
        // in either direction. Move converts explicitly.
        assert!(!is_assignable(
            guard.immut_ref_of(U64_TY),
            guard.mut_ref_of(U64_TY)
        ));
        assert!(!is_assignable(
            guard.mut_ref_of(U64_TY),
            guard.immut_ref_of(U64_TY)
        ));

        // Nested immutable references keep recursing.
        assert!(is_assignable(
            guard.immut_ref_of(guard.immut_ref_of(weak)),
            guard.immut_ref_of(guard.immut_ref_of(strong))
        ));
    });
}

#[test]
fn nominals_from_different_modules_are_distinct() {
    // Interned nominal identity is `(module, name, ty_args)`, which is strictly
    // stronger than the verifier's module-local `StructHandleIndex`: a
    // same-named struct in another module cannot be confused for this one.
    with_context(|guard, module| {
        let name = guard.identifier_of(ident_str!("S"));
        let here = guard.nominal_of(module.id(), name, EMPTY_TYPE_LIST);
        let elsewhere_id = guard.module_id_of(&AccountAddress::TWO, ident_str!("a"));
        let elsewhere = guard.nominal_of(elsewhere_id, name, EMPTY_TYPE_LIST);

        assert!(is_assignable(here, here));
        assert!(!is_assignable(here, elsewhere));
        assert!(!is_assignable(elsewhere, here));
    });
}

#[test]
fn interned_type_list_pointer_equality_is_structural() {
    // `is_assignable` compares a function's arguments and results as whole
    // interned list pointers rather than elementwise; that O(1) path is only
    // sound if list interning deduplicates structurally equal lists.
    with_context(|guard, _module| {
        let first = guard.type_list_of(&[U64_TY, BOOL_TY]);
        let second = guard.type_list_of(&[U64_TY, BOOL_TY]);
        let different = guard.type_list_of(&[BOOL_TY, U64_TY]);
        assert!(
            first == second,
            "structurally equal lists must intern equal"
        );
        assert!(
            first != different,
            "different lists must intern differently"
        );
    });
}
