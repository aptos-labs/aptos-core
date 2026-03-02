// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! NOTE: This is a temporary placeholder until we have the new cached runtime type representation.
//!
//! Local conversion from `SignatureToken` to the runtime `Type` enum.
//!
//! This replicates the logic from `move-vm-runtime/src/loader/type_loader.rs`
//! to avoid a dependency on `move-vm-runtime`.

use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::SignatureToken,
    CompiledModule,
};
use move_vm_types::loaded_data::{
    runtime_types::{AbilityInfo, Type},
    struct_name_indexing::StructNameIndex,
};
use triomphe::Arc as TriompheArc;

/// Convert a single `SignatureToken` to a runtime `Type`.
///
/// `struct_name_table` maps `StructHandleIndex` ordinals to globally unique
/// `StructNameIndex` values. Type parameters become `TyParam(u16)`.
pub(crate) fn convert_sig_token(
    module: &CompiledModule,
    tok: &SignatureToken,
    struct_name_table: &[StructNameIndex],
) -> Type {
    let view = BinaryIndexedView::Module(module);
    convert_impl(&view, tok, struct_name_table)
}

/// Convert a slice of `SignatureToken`s to `Vec<Type>`.
pub(crate) fn convert_sig_tokens(
    module: &CompiledModule,
    toks: &[SignatureToken],
    struct_name_table: &[StructNameIndex],
) -> Vec<Type> {
    toks.iter()
        .map(|t| convert_sig_token(module, t, struct_name_table))
        .collect()
}

fn convert_impl(
    view: &BinaryIndexedView,
    tok: &SignatureToken,
    struct_name_table: &[StructNameIndex],
) -> Type {
    match tok {
        SignatureToken::Bool => Type::Bool,
        SignatureToken::U8 => Type::U8,
        SignatureToken::U16 => Type::U16,
        SignatureToken::U32 => Type::U32,
        SignatureToken::U64 => Type::U64,
        SignatureToken::U128 => Type::U128,
        SignatureToken::U256 => Type::U256,
        SignatureToken::I8 => Type::I8,
        SignatureToken::I16 => Type::I16,
        SignatureToken::I32 => Type::I32,
        SignatureToken::I64 => Type::I64,
        SignatureToken::I128 => Type::I128,
        SignatureToken::I256 => Type::I256,
        SignatureToken::Address => Type::Address,
        SignatureToken::Signer => Type::Signer,
        SignatureToken::TypeParameter(idx) => Type::TyParam(*idx),
        SignatureToken::Vector(inner) => {
            Type::Vector(TriompheArc::new(convert_impl(view, inner, struct_name_table)))
        },
        SignatureToken::Reference(inner) => {
            Type::Reference(Box::new(convert_impl(view, inner, struct_name_table)))
        },
        SignatureToken::MutableReference(inner) => {
            Type::MutableReference(Box::new(convert_impl(view, inner, struct_name_table)))
        },
        SignatureToken::Struct(sh_idx) => {
            let struct_handle = view.struct_handle_at(*sh_idx);
            Type::Struct {
                idx: struct_name_table[sh_idx.0 as usize],
                ability: AbilityInfo::struct_(struct_handle.abilities),
            }
        },
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            let type_args: Vec<Type> = tys
                .iter()
                .map(|t| convert_impl(view, t, struct_name_table))
                .collect();
            let struct_handle = view.struct_handle_at(*sh_idx);
            Type::StructInstantiation {
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
            }
        },
        SignatureToken::Function(args, results, abilities) => {
            let args: Vec<Type> = args
                .iter()
                .map(|t| convert_impl(view, t, struct_name_table))
                .collect();
            let results: Vec<Type> = results
                .iter()
                .map(|t| convert_impl(view, t, struct_name_table))
                .collect();
            Type::Function {
                args,
                results,
                abilities: *abilities,
            }
        },
    }
}
