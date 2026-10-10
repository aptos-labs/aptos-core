// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `AbilityCalculator` behavior that valid Move cannot exercise: malformed
//! inputs, and rules whose callers do not exist yet.

use mono_move_core::{
    abilities::{module_nominal_lookup, AbilityCalculator, AbilityError},
    types::{InternedType, EMPTY_TYPE_LIST, U64_TY},
    Interner, PreparedModule,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_testsuite::with_loaded_module;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
};

const SOURCE: &str = r#"
module 0x1::a {
    struct Res has key, store { x: u64 }
    struct Box<T> has copy, drop, store { x: T }
    // Constraint recorded on the parameter, not on the nominal.
    struct Constrained<T: copy + drop> has copy, drop { x: T }

    public fun touch(_b: &Box<u64>, _c: &Constrained<u64>, _r: &Res) {}
}
"#;

/// Loads the shared test module and runs `check` on its prepared form.
fn with_module(check: impl FnOnce(&ExecutionGuard, &PreparedModule)) {
    with_loaded_module(SOURCE, ident_str!("a"), |guard, module_ir| {
        check(guard, &module_ir.module)
    })
    .expect("module load failed");
}

/// Abilities of `ty` under `module`'s view, with `ty_param_ctx` in scope.
fn abilities_in(
    module: &PreparedModule,
    ty: InternedType,
    ty_param_ctx: &[AbilitySet],
) -> Result<AbilitySet, AbilityError> {
    AbilityCalculator::new(module_nominal_lookup(module), ty_param_ctx).abilities_of(ty)
}

/// Interned type of the nominal named `name`, instantiated at `ty_args`.
fn nominal(
    guard: &ExecutionGuard,
    module: &PreparedModule,
    name: &IdentStr,
    ty_args: &[InternedType],
) -> InternedType {
    guard.nominal_of(
        module.id(),
        guard.identifier_of(name),
        guard.type_list_of(ty_args),
    )
}

/// A reference is copy and drop but never store, whatever it points to.
///
/// Unreachable today: a captured value may not be a reference, and captures are
/// the only caller. It becomes live once slot types are checked.
#[test]
fn references_are_copy_drop_but_never_store() {
    with_module(|guard, module| {
        let res = nominal(guard, module, ident_str!("Res"), &[]);
        for ty in [
            guard.immut_ref_of(U64_TY),
            guard.mut_ref_of(U64_TY),
            // The pointee does not predicate: `&Res` is copy+drop even though
            // `Res` is neither.
            guard.immut_ref_of(res),
        ] {
            assert_eq!(
                abilities_in(module, ty, &[]).expect("reference must resolve"),
                set(&[Ability::Copy, Ability::Drop]),
            );
        }
    });
}

/// A nominal whose argument count disagrees with its declaration is rejected
/// rather than derived from the shorter of the two.
///
/// The derivation zips parameters against arguments, and `zip` stops at the
/// shorter side, so without this check a mismatch would silently yield
/// abilities computed from a prefix.
#[test]
fn type_argument_arity_mismatch_is_rejected_not_truncated() {
    with_module(|guard, module| {
        // `Box` declares one parameter; hand it two.
        let bogus = nominal(guard, module, ident_str!("Box"), &[U64_TY, U64_TY]);
        let err = abilities_in(module, bogus, &[]).unwrap_err();
        assert!(
            matches!(err, AbilityError::TypeArgumentArityMismatch {
                declared: 1,
                actual: 2
            }),
            "unexpected error: {err}"
        );
    });
}

/// A nominal the module cannot name, and a type parameter outside the bound
/// scope, are both errors rather than defaults.
#[test]
fn unresolvable_inputs_are_errors() {
    with_module(|guard, module| {
        let elsewhere = guard.module_id_of(&AccountAddress::TWO, ident_str!("elsewhere"));
        let foreign = guard.nominal_of(
            elsewhere,
            guard.identifier_of(ident_str!("Res")),
            EMPTY_TYPE_LIST,
        );
        assert!(matches!(
            abilities_in(module, foreign, &[]).unwrap_err(),
            AbilityError::UnknownNominal
        ));

        let one_param = [set(&[Ability::Copy])];
        assert!(matches!(
            abilities_in(module, guard.type_param_of(3), &one_param).unwrap_err(),
            AbilityError::TypeParamOutOfRange {
                idx: 3,
                num_params: 1
            }
        ));
        assert!(matches!(
            abilities_in(module, guard.type_param_of(0), &[]).unwrap_err(),
            AbilityError::TypeParamOutOfRange { .. }
        ));
    });
}

/// Per-parameter ability constraints are recorded on the handle.
///
/// Ability derivation reads only `is_phantom`, so nothing else would notice if
/// these were dropped; they are stored for type-argument constraint checking.
#[test]
fn type_parameter_constraints_are_recorded() {
    with_module(|guard, module| {
        let handle = module
            .nominal_handle(module.id(), guard.identifier_of(ident_str!("Constrained")))
            .expect("nominal not in table");
        assert_eq!(
            handle.type_parameters[0].constraints,
            set(&[Ability::Copy, Ability::Drop])
        );
    });
}

fn set(abilities: &[Ability]) -> AbilitySet {
    abilities
        .iter()
        .fold(AbilitySet::EMPTY, |acc, ability| acc | *ability)
}
