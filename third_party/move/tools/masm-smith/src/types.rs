// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type helpers: equality, nesting depth, substitution of type parameters, and inference of type
//! arguments.

use crate::{
    abilities::{type_abilities, AbilityCtx},
    error::Result,
    ir::{QualifiedStruct, StructDecl, Type, TypeParam},
};
use arbitrary::Unstructured;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    language_storage::ModuleId,
};

/// Maximum nesting depth of a type. Primitives are depth 1; `vector<T>` is depth `1 +
/// depth(T)`; `Struct<T1, ..>` is `1 + max(depth(Ti))`; references and function types behave
/// analogously.
pub fn type_depth(t: &Type) -> usize {
    match t {
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Address
        | Type::Signer
        | Type::TypeParam(_) => 1,
        Type::Vector(inner) => 1 + type_depth(inner),
        Type::Reference(_, inner) => 1 + type_depth(inner),
        Type::Struct(_, args) => 1 + args.iter().map(type_depth).max().unwrap_or(0),
        Type::Function { args, results, .. } => {
            1 + args
                .iter()
                .chain(results.iter())
                .map(type_depth)
                .max()
                .unwrap_or(0)
        },
    }
}

/// Substitute type parameters in `t` using `args`. Out-of-range parameters are left as-is so
/// callers can spot the mistake during testing.
pub fn substitute(t: &Type, args: &[Type]) -> Type {
    match t {
        Type::TypeParam(i) => args
            .get(*i as usize)
            .cloned()
            .unwrap_or(Type::TypeParam(*i)),
        Type::Vector(inner) => Type::Vector(Box::new(substitute(inner, args))),
        Type::Reference(is_mut, inner) => {
            Type::Reference(*is_mut, Box::new(substitute(inner, args)))
        },
        Type::Struct(qs, type_args) => Type::Struct(
            qs.clone(),
            type_args.iter().map(|ta| substitute(ta, args)).collect(),
        ),
        Type::Function {
            args: fa,
            results,
            abilities,
        } => Type::Function {
            args: fa.iter().map(|ta| substitute(ta, args)).collect(),
            results: results.iter().map(|ta| substitute(ta, args)).collect(),
            abilities: *abilities,
        },
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Address
        | Type::Signer => t.clone(),
    }
}

/// Returns whether the type is a reference (immutable or mutable).
pub fn is_reference(t: &Type) -> bool {
    matches!(t, Type::Reference(_, _))
}

/// Returns whether `t` may instantiate a type parameter. Mirrors the signature checker:
/// references are only allowed as the immediate parameter/result types of a function type.
pub fn is_valid_type_arg(t: &Type) -> bool {
    match t {
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Address
        | Type::Signer
        | Type::TypeParam(_) => true,
        Type::Reference(_, _) => false,
        Type::Vector(inner) => is_valid_type_arg(inner),
        Type::Struct(_, args) => args.iter().all(is_valid_type_arg),
        Type::Function { args, results, .. } => args
            .iter()
            .chain(results)
            .all(|ty| is_valid_type_arg(deref(ty).unwrap_or(ty))),
    }
}

/// Returns whether type parameter `idx` occurs anywhere in `t`.
pub fn mentions_type_param(t: &Type, idx: u16) -> bool {
    t.any(&|ty| matches!(ty, Type::TypeParam(found) if *found == idx))
}

/// Returns whether a function type occurs anywhere in `t`. Struct fields are not inspected:
/// generated fields hold function values only through the struct's type arguments.
pub fn mentions_function(t: &Type) -> bool {
    t.any(&|ty| matches!(ty, Type::Function { .. }))
}

/// Returns the inner referenced type if `t` is a reference.
pub fn deref(t: &Type) -> Option<&Type> {
    if let Type::Reference(_, inner) = t {
        Some(inner)
    } else {
        None
    }
}

/// Returns true if `t` is `&signer` — the shape entry parameters use for transaction
/// signers. Mutable signer refs aren't accepted by the transactional runner's `--signers`
/// flag, so this is `&signer` only, not `&mut signer`.
pub fn is_signer_ref(t: &Type) -> bool {
    matches!(t, Type::Reference(false, inner) if matches!(**inner, Type::Signer))
}

/// Returns whether `t` is a signed or unsigned integer type of any width.
pub fn is_integer(t: &Type) -> bool {
    is_unsigned_integer(t) || is_signed_integer(t)
}

/// Returns true if `t` is one of `u8`/`u16`/`u32`/`u64`/`u128`/`u256`.
pub fn is_unsigned_integer(t: &Type) -> bool {
    matches!(
        t,
        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256
    )
}

/// Returns true if `t` is one of `i8`/`i16`/`i32`/`i64`/`i128`/`i256`.
pub fn is_signed_integer(t: &Type) -> bool {
    matches!(
        t,
        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::I256
    )
}

/// Returns a struct type's qualified name, excluding its type arguments.
pub fn struct_name(t: &Type) -> Option<&QualifiedStruct> {
    if let Type::Struct(qs, _) = t {
        Some(qs)
    } else {
        None
    }
}

/// Unify a `raw` type pattern (potentially containing `TypeParam(i)`) against a `concrete`
/// type, updating `bindings`. Returns `false` if no consistent binding exists.
///
/// Used by [`infer_type_args`] to back-derive a struct's type arguments from the concrete
/// types observed on the operand stack at a `pack` site. Bindings are write-once: the first
/// occurrence of `TypeParam(i)` fixes it; subsequent occurrences must equal the prior value.
fn unify(raw: &Type, concrete: &Type, bindings: &mut [Option<Type>]) -> bool {
    match raw {
        Type::TypeParam(index) => {
            let Some(binding) = bindings.get_mut(usize::from(*index)) else {
                return false;
            };
            match binding {
                None => {
                    *binding = Some(concrete.clone());
                    true
                },
                Some(existing) => existing == concrete,
            }
        },
        Type::Vector(raw_elem) => {
            matches!(concrete, Type::Vector(elem) if unify(raw_elem, elem, bindings))
        },
        Type::Reference(raw_mut, raw_inner) => matches!(
            concrete,
            Type::Reference(is_mut, inner) if raw_mut == is_mut && unify(raw_inner, inner, bindings)
        ),
        Type::Struct(raw_qs, raw_args) => matches!(
            concrete,
            Type::Struct(qs, args) if raw_qs == qs
                && raw_args.len() == args.len()
                && raw_args.iter().zip(args).all(|(raw_arg, arg)| unify(raw_arg, arg, bindings))
        ),
        // Type parameters inside a function type are not inferred.
        Type::Function { .. }
        | Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Address
        | Type::Signer => raw == concrete,
    }
}

/// Given a struct or function's declared `type_params` and the raw IR types of its "consumed
/// slots" (e.g., field types for a `pack`), back-derive concrete type arguments from the
/// `stack_slice` (the bottom-up sequence of operand-stack entries matched against those
/// slots).
///
/// For unbound parameters (i.e., none of the raw slots referenced them), if the parameter is
/// declared `is_phantom`, we draw a depth-bounded concrete type via [`pick_concrete_type`]
/// (which may pick a nested in-module struct); otherwise we return `None` (the candidate
/// doesn't match).
///
/// Returns `None` if no consistent binding exists or if a non-phantom parameter remained
/// unbound after walking the raw slots.
pub fn infer_type_args(
    type_params: &[TypeParam],
    raw_slots: &[Type],
    stack_slice: &[Type],
    u: &mut Unstructured<'_>,
    structs: &[StructDecl],
    external_modules: &[crate::ir::Module],
    module_id: &ModuleId,
    max_depth: u8,
) -> Result<Option<Vec<Type>>> {
    if raw_slots.len() != stack_slice.len() {
        return Ok(None);
    }
    let mut bindings: Vec<Option<Type>> = vec![None; type_params.len()];
    for (raw, concrete) in raw_slots.iter().zip(stack_slice) {
        if !unify(raw, concrete, &mut bindings) {
            return Ok(None);
        }
    }
    let mut args = Vec::with_capacity(type_params.len());
    for (i, b) in bindings.into_iter().enumerate() {
        match b {
            Some(t) => args.push(t),
            None if type_params[i].is_phantom => {
                args.push(pick_concrete_type(
                    u,
                    structs,
                    external_modules,
                    module_id,
                    type_params[i].constraints,
                    max_depth,
                )?);
            },
            None => return Ok(None),
        }
    }
    Ok(Some(args))
}

/// Draw a concrete primitive type from `u`. Used as the type-arg picker for call sites
/// (where types must be synthesizable) and as the fallback for phantom-parameter slots.
pub fn pick_concrete_primitive(u: &mut Unstructured<'_>) -> Result<Type> {
    let choice = u.int_in_range(0u8..=13)?;
    Ok(match choice {
        0 => Type::U64,
        1 => Type::U8,
        2 => Type::Bool,
        3 => Type::Address,
        4 => Type::U16,
        5 => Type::U32,
        6 => Type::U128,
        7 => Type::U256,
        8 => Type::I8,
        9 => Type::I16,
        10 => Type::I32,
        11 => Type::I64,
        12 => Type::I128,
        _ => Type::I256,
    })
}

/// Draw a vector of `n` concrete primitive types via [`pick_concrete_primitive`].
pub fn pick_concrete_primitives(u: &mut Unstructured<'_>, n: usize) -> Result<Vec<Type>> {
    (0..n).map(|_| pick_concrete_primitive(u)).collect()
}

/// Picks a primitive or a local or external struct, nesting up to `depth_budget` levels.
/// Struct candidates must satisfy `required` after instantiation. The fallback is a
/// primitive, so callers must restrict `required` to a subset of `copy + drop + store`
/// to guarantee it is satisfied.
///
/// Candidate filtering:
///   - The struct's *declared* ability set must satisfy `required`. (Phantom-vs-non-phantom
///     subtleties — where the type-arg's own abilities dilute the result — are checked
///     after we've assembled the candidate by computing its instantiated abilities and
///     falling back to a primitive if the check fails.)
///   - We exclude `key` structs unless `required` itself demands `key`. They have narrower
///     applicability; recursively picking them rarely matches caller's needs.
pub fn pick_concrete_type(
    u: &mut Unstructured<'_>,
    structs: &[StructDecl],
    external_modules: &[crate::ir::Module],
    module_id: &ModuleId,
    required: AbilitySet,
    depth_budget: u8,
) -> Result<Type> {
    let any_struct = !structs.is_empty() || external_modules.iter().any(|m| !m.structs.is_empty());
    let want_struct = depth_budget > 0 && any_struct && u.arbitrary::<bool>()?;
    if want_struct {
        // Combined candidate pool: local structs (carrying the current `module_id`) plus
        // each foreign module's structs (carrying that module's id). Foreign structs are
        // legal in type positions — params, returns, locals, field types, type args. The
        // bytecode opcodes that take a `StructDefinitionIndex` (pack / unpack /
        // borrow_field / move_to / move_from / *borrow_global*) stay local-only: the format
        // cannot express them for a foreign struct.
        let mut all: Vec<(ModuleId, &StructDecl)> = Vec::new();
        for s in structs {
            all.push((module_id.clone(), s));
        }
        for m in external_modules {
            for s in &m.structs {
                all.push((m.id.clone(), s));
            }
        }
        let candidates: Vec<(ModuleId, &StructDecl)> = all
            .into_iter()
            .filter(|(_, d)| required.is_subset(d.abilities))
            .filter(|(_, d)| {
                !d.abilities.has_ability(Ability::Key) || required.has_ability(Ability::Key)
            })
            .collect();
        if !candidates.is_empty() {
            let (pick_mid, pick_decl) = u.choose(&candidates)?.clone();
            let pick = pick_decl.clone();
            let mut type_args: Vec<Type> = Vec::with_capacity(pick.type_params.len());
            for tp in &pick.type_params {
                let arg = pick_concrete_type(
                    u,
                    structs,
                    external_modules,
                    module_id,
                    tp.constraints,
                    depth_budget.saturating_sub(1),
                )?;
                type_args.push(arg);
            }
            let candidate = Type::Struct(
                QualifiedStruct {
                    module: pick_mid,
                    name: pick.name.clone(),
                },
                type_args,
            );
            let ctx = AbilityCtx {
                local_module_id: Some(module_id),
                structs,
                external_modules,
                type_param_constraints: &[],
            };
            if required.is_subset(type_abilities(&candidate, &ctx)) {
                return Ok(candidate);
            }
            // The instantiated abilities (with the picked type-args bound) failed to satisfy
            // the requirement — fall through to a primitive, which trivially satisfies any
            // subset of {copy, drop, store}.
        }
    }
    pick_concrete_primitive(u)
}

/// Pick an ability-set to declare on a type parameter. Half the time it is
/// `copy + drop + store` (which any primitive type satisfies, so the dispatcher finds
/// candidates freely); the other half we draw a non-empty subset
/// of `{copy, drop, store}`. `key` is intentionally excluded — only key-able structs satisfy
/// it, and our type-arg pickers don't routinely produce those, so including `key` would
/// starve `try_pack`/`try_call` of viable candidates.
pub fn pick_type_param_constraint(u: &mut Unstructured<'_>) -> Result<AbilitySet> {
    if u.arbitrary::<bool>()? {
        return Ok(AbilitySet::PRIMITIVES);
    }
    let mut set = AbilitySet::EMPTY;
    if u.arbitrary::<bool>()? {
        set = set.add(Ability::Copy);
    }
    if u.arbitrary::<bool>()? {
        set = set.add(Ability::Drop);
    }
    if u.arbitrary::<bool>()? {
        set = set.add(Ability::Store);
    }
    // Keep every generated constraint set nonempty.
    if set.is_empty() {
        set = set.add(Ability::Drop);
    }
    Ok(set)
}

/// Number of account addresses tests draw from: `0x1..=0x4`. Signers and the addresses of
/// global operations share the pool, so later transactions find resources earlier ones stored.
pub const ACCOUNT_POOL_SIZE: u8 = 4;

/// An account address for a signer, a global operation, or an address value. Usually one of
/// the shared pool; occasionally an arbitrary address that holds no resources.
pub fn pick_account_address(u: &mut Unstructured<'_>) -> Result<AccountAddress> {
    if crate::violations::chance(u, 32)? {
        return Ok(AccountAddress::new(u.arbitrary()?));
    }
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[AccountAddress::LENGTH - 1] = u.int_in_range(1..=ACCOUNT_POOL_SIZE)?;
    Ok(AccountAddress::new(bytes))
}
