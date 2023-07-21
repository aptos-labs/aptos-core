// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    binary_views::BinaryIndexedView, errors::PartialVMResult, file_format::SignatureToken,
};
use move_core_types::{identifier::IdentStr, language_storage::ModuleId};
use move_vm_types::loaded_data::runtime_types::{CachedStructIndex, StructType, Type};
use std::sync::Arc;

// `make_type_internal` returns a `Type` given a signature and a resolver which
// is resonsible to map a local struct index to a global one
pub fn make_type_internal<F>(
    module: BinaryIndexedView,
    tok: &SignatureToken,
    resolver: &F,
) -> PartialVMResult<Type>
where
    F: Fn(&IdentStr, &ModuleId) -> PartialVMResult<(CachedStructIndex, Arc<StructType>)>,
{
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
            let inner_type = make_type_internal(module, inner_tok, resolver)?;
            Type::Vector(Box::new(inner_type))
        },
        SignatureToken::Reference(inner_tok) => {
            let inner_type = make_type_internal(module, inner_tok, resolver)?;
            Type::Reference(Box::new(inner_type))
        },
        SignatureToken::MutableReference(inner_tok) => {
            let inner_type = make_type_internal(module, inner_tok, resolver)?;
            Type::MutableReference(Box::new(inner_type))
        },
        SignatureToken::Struct(sh_idx) => {
            let struct_handle = module.struct_handle_at(*sh_idx);
            let struct_name = module.identifier_at(struct_handle.name);
            let module_handle = module.module_handle_at(struct_handle.module);
            let module_id = ModuleId::new(
                *module.address_identifier_at(module_handle.address),
                module.identifier_at(module_handle.name).to_owned(),
            );
            let (def_idx, struct_) = resolver(struct_name, &module_id)?;
            Type::Struct {
                index: def_idx,
                ability: struct_.abilities,
            }
        },
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            let type_parameters: Vec<_> = tys
                .iter()
                .map(|tok| make_type_internal(module, tok, resolver))
                .collect::<PartialVMResult<_>>()?;
            let struct_handle = module.struct_handle_at(*sh_idx);
            let struct_name = module.identifier_at(struct_handle.name);
            let module_handle = module.module_handle_at(struct_handle.module);
            let module_id = ModuleId::new(
                *module.address_identifier_at(module_handle.address),
                module.identifier_at(module_handle.name).to_owned(),
            );
            let (def_idx, struct_) = resolver(struct_name, &module_id)?;
            Type::StructInstantiation {
                index: def_idx,
                base_ability_set: struct_.abilities,
                ty_args: type_parameters,
                phantom_ty_args_mask: struct_
                    .type_parameters
                    .iter()
                    .map(|ty| ty.is_phantom)
                    .collect(),
            }
        },
    };
    Ok(res)
}
