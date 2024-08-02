// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format as FF,
    file_format::TableIndex,
};
use move_core_types::vm_status::StatusCode;
use move_vm_types::loaded_data::{
    runtime_access_specifier::{
        AccessSpecifier, AccessSpecifierClause, AddressSpecifier, AddressSpecifierFunction,
        ResourceSpecifier,
    },
    runtime_types::{StructIdentifier, Type},
};

/// Loads an access specifier from the file format into the runtime representation.
pub fn load_access_specifier(
    module: BinaryIndexedView,
    signature_table: &[Vec<Type>],
    struct_names: &[StructIdentifier],
    specifier: &Option<Vec<FF::AccessSpecifier>>,
) -> PartialVMResult<AccessSpecifier> {
    if let Some(specs) = specifier {
        let mut incls = vec![];
        let mut excls = vec![];
        for spec in specs {
            let resource =
                load_resource_specifier(module, signature_table, struct_names, &spec.resource)?;
            let address = load_address_specifier(module, &spec.address)?;
            let clause = AccessSpecifierClause {
                kind: spec.kind,
                resource,
                address,
            };
            if spec.negated {
                excls.push(clause)
            } else {
                incls.push(clause)
            }
        }
        Ok(AccessSpecifier::Constraint(incls, excls))
    } else {
        Ok(AccessSpecifier::Any)
    }
}

fn load_resource_specifier(
    module: BinaryIndexedView,
    signature_table: &[Vec<Type>],
    struct_names: &[StructIdentifier],
    spec: &FF::ResourceSpecifier,
) -> PartialVMResult<ResourceSpecifier> {
    use FF::ResourceSpecifier::*;
    match spec {
        Any => Ok(ResourceSpecifier::Any),
        DeclaredAtAddress(addr_idx) => Ok(ResourceSpecifier::DeclaredAtAddress(*access_table(
            module.address_identifiers(),
            addr_idx.0,
        )?)),
        DeclaredInModule(mod_idx) => Ok(ResourceSpecifier::DeclaredInModule(
            module
                .safe_module_id_for_handle(access_table(module.module_handles(), mod_idx.0)?)
                .ok_or_else(index_out_of_range)?,
        )),
        Resource(str_idx) => Ok(ResourceSpecifier::Resource(
            access_table(struct_names, str_idx.0)?.clone(),
        )),
        ResourceInstantiation(str_idx, ty_idx) => Ok(ResourceSpecifier::ResourceInstantiation(
            access_table(struct_names, str_idx.0)?.clone(),
            access_table(signature_table, ty_idx.0)?.clone(),
        )),
    }
}

fn load_address_specifier(
    module: BinaryIndexedView,
    spec: &FF::AddressSpecifier,
) -> PartialVMResult<AddressSpecifier> {
    use FF::AddressSpecifier::*;
    match spec {
        Any => Ok(AddressSpecifier::Any),
        Literal(idx) => Ok(AddressSpecifier::Literal(*access_table(
            module.address_identifiers(),
            idx.0,
        )?)),
        Parameter(param, fun) => {
            let fun = if let Some(idx) = fun {
                let fun_inst = access_table(module.function_instantiations(), idx.0)?;
                let fun_handle = access_table(module.function_handles(), fun_inst.handle.0)?;
                let mod_handle = access_table(module.module_handles(), fun_handle.module.0)?;
                let mod_id = module
                    .safe_module_id_for_handle(mod_handle)
                    .ok_or_else(index_out_of_range)?;
                let mod_name = mod_id.short_str_lossless();
                let fun_name = access_table(module.identifiers(), fun_handle.name.0)?;
                AddressSpecifierFunction::parse(&mod_name, fun_name.as_str()).ok_or_else(|| {
                    PartialVMError::new(StatusCode::ACCESS_CONTROL_INVARIANT_VIOLATION)
                        .with_message(format!(
                            "function `{}::{}` not supported for address specifier",
                            mod_name, fun_name
                        ))
                })?
            } else {
                AddressSpecifierFunction::Identity
            };
            Ok(AddressSpecifier::Eval(fun, *param))
        },
    }
}

fn access_table<T>(table: &[T], idx: TableIndex) -> PartialVMResult<&T> {
    if (idx as usize) < table.len() {
        Ok(&table[idx as usize])
    } else {
        Err(index_out_of_range())
    }
}

fn index_out_of_range() -> PartialVMError {
    PartialVMError::new(StatusCode::ACCESS_CONTROL_INVARIANT_VIOLATION)
        .with_message("table index out of range".to_owned())
}
