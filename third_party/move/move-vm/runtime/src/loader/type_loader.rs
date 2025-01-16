// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::SignatureToken,
};
use move_core_types::vm_status::StatusCode;
use move_vm_types::loaded_data::runtime_types::{AbilityInfo, StructNameIndex, Type};
use triomphe::Arc as TriompheArc;

/// Converts a signature token into the in memory type representation used by the MoveVM.
pub fn intern_type(
    module: BinaryIndexedView,
    tok: &SignatureToken,
    struct_name_table: &[StructNameIndex],
) -> PartialVMResult<Type> {
    let res = match tok {
        SignatureToken::Bool => Type::Bool,
        SignatureToken::U8 => Type::U8,
        SignatureToken::U16 => Type::U16,
        SignatureToken::U32 => Type::U32,
        SignatureToken::U64 => Type::U64,
        SignatureToken::U128 => Type::U128,
        SignatureToken::U256 => Type::U256,
        SignatureToken::Address => Type::Address,
        SignatureToken::Signer => Type::Signer,
        SignatureToken::TypeParameter(idx) => Type::TyParam(*idx),
        SignatureToken::Vector(inner_tok) => {
            let inner_type = intern_type(module, inner_tok, struct_name_table)?;
            Type::Vector(TriompheArc::new(inner_type))
        },
        SignatureToken::Function(..) => {
            // TODO: implement closures
            return Err(PartialVMError::new(StatusCode::UNIMPLEMENTED_FEATURE)
                .with_message("function types in the type loader".to_owned()));
        },
        SignatureToken::Reference(inner_tok) => {
            let inner_type = intern_type(module, inner_tok, struct_name_table)?;
            Type::Reference(Box::new(inner_type))
        },
        SignatureToken::MutableReference(inner_tok) => {
            let inner_type = intern_type(module, inner_tok, struct_name_table)?;
            Type::MutableReference(Box::new(inner_type))
        },
        SignatureToken::Struct(sh_idx) => {
            let struct_handle = module.struct_handle_at(*sh_idx);
            Type::Struct {
                idx: struct_name_table[sh_idx.0 as usize],
                ability: AbilityInfo::struct_(struct_handle.abilities),
            }
        },
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            let type_args: Vec<_> = tys
                .iter()
                .map(|tok| intern_type(module, tok, struct_name_table))
                .collect::<PartialVMResult<_>>()?;
            let struct_handle = module.struct_handle_at(*sh_idx);
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
    };
    Ok(res)
}
