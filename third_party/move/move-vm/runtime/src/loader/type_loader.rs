// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    binary_views::BinaryIndexedView, errors::PartialVMResult, file_format::SignatureToken,
};
use move_vm_types::loaded_data::{
    runtime_types::{AbilityInfo, Type},
    struct_name_indexing::StructNameIndex,
};
use triomphe::Arc as TriompheArc;

/// Converts a signature token into the in memory type representation used by the MoveVM.
pub fn intern_type(
    module: BinaryIndexedView,
    tok: &SignatureToken,
    struct_name_table: &[StructNameIndex],
) -> PartialVMResult<Type> {
    Ok(intern_type_impl(module, tok, struct_name_table)?.0)
}

/// Converts a stream of signature tokens into the in memory type representation used by the
/// MoveVM. Also returns true if all tokens are non-generic (contain 0 type parameters).
pub fn intern_types(
    module: BinaryIndexedView,
    toks: &[SignatureToken],
    struct_name_table: &[StructNameIndex],
) -> PartialVMResult<(Vec<Type>, bool)> {
    intern_types_impl(module, toks, struct_name_table)
}

fn intern_types_impl(
    module: BinaryIndexedView,
    toks: &[SignatureToken],
    struct_name_table: &[StructNameIndex],
) -> PartialVMResult<(Vec<Type>, bool)> {
    let mut tys = Vec::with_capacity(toks.len());
    let mut all_fully_instantiated = true;
    for tok in toks {
        let (ty, is_fully_instantiated) = intern_type_impl(module, tok, struct_name_table)?;
        tys.push(ty);
        all_fully_instantiated &= is_fully_instantiated;
    }
    Ok((tys, all_fully_instantiated))
}

/// Returns the type, and true if this type is fully instantiated, and false otherwise.
fn intern_type_impl(
    module: BinaryIndexedView,
    tok: &SignatureToken,
    struct_name_table: &[StructNameIndex],
) -> PartialVMResult<(Type, bool)> {
    let res = match tok {
        SignatureToken::Bool => (Type::Bool, true),
        SignatureToken::U8 => (Type::U8, true),
        SignatureToken::U16 => (Type::U16, true),
        SignatureToken::U32 => (Type::U32, true),
        SignatureToken::U64 => (Type::U64, true),
        SignatureToken::U128 => (Type::U128, true),
        SignatureToken::U256 => (Type::U256, true),
        SignatureToken::Address => (Type::Address, true),
        SignatureToken::Signer => (Type::Signer, true),
        SignatureToken::TypeParameter(idx) => (Type::TyParam(*idx), false),
        SignatureToken::Vector(inner_tok) => {
            let (inner_type, is_fully_instantiated) =
                intern_type_impl(module, inner_tok, struct_name_table)?;
            (
                Type::Vector(TriompheArc::new(inner_type)),
                is_fully_instantiated,
            )
        },
        SignatureToken::Function(args, results, abilities) => {
            let (args, args_fully_instantiated) =
                intern_types_impl(module, args, struct_name_table)?;
            let (results, results_fully_instantiated) =
                intern_types_impl(module, results, struct_name_table)?;
            let ty = Type::Function {
                args,
                results,
                abilities: *abilities,
            };
            (ty, args_fully_instantiated && results_fully_instantiated)
        },
        SignatureToken::Reference(inner_tok) => {
            let (inner_type, is_fully_instantiated) =
                intern_type_impl(module, inner_tok, struct_name_table)?;
            (Type::Reference(Box::new(inner_type)), is_fully_instantiated)
        },
        SignatureToken::MutableReference(inner_tok) => {
            let (inner_type, is_fully_instantiated) =
                intern_type_impl(module, inner_tok, struct_name_table)?;
            (
                Type::MutableReference(Box::new(inner_type)),
                is_fully_instantiated,
            )
        },
        SignatureToken::Struct(sh_idx) => {
            let struct_handle = module.struct_handle_at(*sh_idx);
            let ty = Type::Struct {
                idx: struct_name_table[sh_idx.0 as usize],
                ability: AbilityInfo::struct_(struct_handle.abilities),
            };
            (ty, true)
        },
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            let (type_args, type_args_fully_instantiated) =
                intern_types_impl(module, tys, struct_name_table)?;
            let struct_handle = module.struct_handle_at(*sh_idx);
            let ty = Type::StructInstantiation {
                idx: struct_name_table[sh_idx.0 as usize],
                ty_args: TriompheArc::new(type_args),
                ability: AbilityInfo::generic_struct(
                    struct_handle.abilities,
                    struct_handle
                        .type_parameters
                        .iter()
                        .map(|ty| ty.is_phantom)
                        .collect(),
                ),
            };
            (ty, type_args_fully_instantiated)
        },
    };
    Ok(res)
}
