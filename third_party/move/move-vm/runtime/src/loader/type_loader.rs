// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    binary_views::BinaryIndexedView, errors::PartialVMResult, file_format::SignatureToken,
};
use move_vm_types::loaded_data::runtime_types::{AbilityInfo, StructNameIndex, Type, TypeBuilder};

/// Converts a signature token into in-memory type representation used by the MoveVM.
pub fn create_ty_from_sig_token(
    module: BinaryIndexedView,
    tok: &SignatureToken,
    struct_name_table: &[StructNameIndex],
) -> PartialVMResult<Type> {
    use SignatureToken::*;

    Ok(match tok {
        Bool => TypeBuilder::create_bool_ty(),
        U8 => TypeBuilder::create_u8_ty(),
        U16 => TypeBuilder::create_u16_ty(),
        U32 => TypeBuilder::create_u32_ty(),
        U64 => TypeBuilder::create_u64_ty(),
        U128 => TypeBuilder::create_u128_ty(),
        U256 => TypeBuilder::create_u256_ty(),
        Address => TypeBuilder::create_address_ty(),
        Signer => TypeBuilder::create_signer_ty(),
        TypeParameter(idx) => Type::TyParam(*idx),
        Vector(elem_tok) => {
            let elem_ty = create_ty_from_sig_token(module, elem_tok, struct_name_table)?;
            TypeBuilder::create_vector_ty(elem_ty)?
        },
        Reference(inner_tok) => {
            let inner_ty = create_ty_from_sig_token(module, inner_tok, struct_name_table)?;
            TypeBuilder::create_reference_ty(inner_ty)?
        },
        MutableReference(inner_tok) => {
            let inner_ty = create_ty_from_sig_token(module, inner_tok, struct_name_table)?;
            TypeBuilder::create_mut_reference_ty(inner_ty)?
        },
        Struct(sh_idx) => {
            let idx = struct_name_table[sh_idx.0 as usize];
            let struct_handle = module.struct_handle_at(*sh_idx);
            let ability = AbilityInfo::struct_(struct_handle.abilities);
            TypeBuilder::create_struct_ty(idx, ability)
        },
        StructInstantiation(sh_idx, toks) => {
            let ty_args: Vec<_> = toks
                .iter()
                .map(|tok| create_ty_from_sig_token(module, tok, struct_name_table))
                .collect::<PartialVMResult<_>>()?;

            let idx = struct_name_table[sh_idx.0 as usize];
            let struct_handle = module.struct_handle_at(*sh_idx);
            let ability = AbilityInfo::generic_struct(
                struct_handle.abilities,
                struct_handle
                    .type_parameters
                    .iter()
                    .map(|ty| ty.is_phantom)
                    .collect(),
            );
            TypeBuilder::create_struct_instantiation_ty(idx, ability, ty_args)?
        },
    })
}
