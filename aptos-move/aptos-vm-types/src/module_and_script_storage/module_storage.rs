// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValueMetadata;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_runtime::{LoadedFunction, LoadedFunctionOwner, ModuleStorage};
use move_vm_types::loaded_data::runtime_types::Type;
use std::collections::{btree_map, BTreeMap};

/// Represents module storage used by the Aptos blockchain.
pub trait AptosModuleStorage: ModuleStorage {
    /// Returns the state value metadata associated with this module. The error is returned if
    /// there is a storage error. If the module does not exist, [None] is returned.
    fn fetch_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>>;

    fn load_function_with_type_arg_inference(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        expected_return_ty: &Type,
    ) -> VMResult<LoadedFunction> {
        let (module, function) =
            self.fetch_function_definition(module_id.address(), module_id.name(), function_name)?;

        if function.return_tys().len() != 1 {
            // For functions that are marked constructor this should not happen.
            return Err(PartialVMError::new(StatusCode::ABORTED).finish(Location::Undefined));
        }

        let mut map = BTreeMap::new();
        if !match_return_type(&function.return_tys()[0], expected_return_ty, &mut map) {
            // For functions that are marked constructor this should not happen.
            return Err(
                PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                    .finish(Location::Undefined),
            );
        }

        // Construct the type arguments from the match.
        let num_ty_args = function.ty_param_abilities().len();
        let mut ty_args = Vec::with_capacity(num_ty_args);
        for i in 0..num_ty_args {
            if let Some(t) = map.get(&(i as u16)) {
                ty_args.push((*t).clone());
            } else {
                // Unknown type argument we are not able to infer the type arguments.
                // For functions that are marked constructor this should not happen.
                return Err(
                    PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                        .finish(Location::Undefined),
                );
            }
        }

        Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
            .map_err(|e| e.finish(Location::Module(module_id.clone())))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            function,
        })
    }
}

/// Matches the actual returned type to the expected type, binding any type args to the necessary
/// type as stored in the map. The expected type must be a concrete type (no [Type::TyParam]).
/// Returns true if a successful match is made.
// TODO: is this really needed in presence of paranoid mode? This does a deep structural
//   comparison and is expensive.
fn match_return_type<'a>(
    returned: &Type,
    expected: &'a Type,
    map: &mut BTreeMap<u16, &'a Type>,
) -> bool {
    match (returned, expected) {
        // The important case, deduce the type params.
        (Type::TyParam(idx), _) => match map.entry(*idx) {
            btree_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(expected);
                true
            },
            btree_map::Entry::Occupied(occupied_entry) => *occupied_entry.get() == expected,
        },
        // Recursive types we need to recurse the matching types.
        (Type::Reference(ret_inner), Type::Reference(expected_inner))
        | (Type::MutableReference(ret_inner), Type::MutableReference(expected_inner)) => {
            match_return_type(ret_inner, expected_inner, map)
        },
        (Type::Vector(ret_inner), Type::Vector(expected_inner)) => {
            match_return_type(ret_inner, expected_inner, map)
        },
        // Function types, the expected abilities need to be equal to the provided ones,
        // and recursively argument and result types need to match.
        (
            Type::Function {
                args,
                results,
                abilities,
            },
            Type::Function {
                args: exp_args,
                results: exp_results,
                abilities: exp_abilities,
            },
        ) if abilities == exp_abilities
            && args.len() == exp_args.len()
            && results.len() == exp_results.len() =>
        {
            args.iter()
                .zip(exp_args)
                .all(|(t, e)| match_return_type(t, e, map))
                && results
                    .iter()
                    .zip(exp_results)
                    .all(|(t, e)| match_return_type(t, e, map))
        },
        // Abilities should not contribute to the equality check as they just serve for caching
        // computations. For structs the both need to be the same struct.
        (
            Type::Struct { idx: ret_idx, .. },
            Type::Struct {
                idx: expected_idx, ..
            },
        ) => *ret_idx == *expected_idx,
        // For struct instantiations we need to additionally match all type arguments.
        (
            Type::StructInstantiation {
                idx: ret_idx,
                ty_args: ret_fields,
                ..
            },
            Type::StructInstantiation {
                idx: expected_idx,
                ty_args: expected_fields,
                ..
            },
        ) => {
            *ret_idx == *expected_idx
                && ret_fields.len() == expected_fields.len()
                && ret_fields
                    .iter()
                    .zip(expected_fields.iter())
                    .all(|types| match_return_type(types.0, types.1, map))
        },
        // For primitive types we need to assure the types match.
        (Type::U8, Type::U8)
        | (Type::U16, Type::U16)
        | (Type::U32, Type::U32)
        | (Type::U64, Type::U64)
        | (Type::U128, Type::U128)
        | (Type::U256, Type::U256)
        | (Type::Bool, Type::Bool)
        | (Type::Address, Type::Address)
        | (Type::Signer, Type::Signer) => true,
        // Otherwise the types do not match, and we can't match return type to the expected type.
        // Note we don't use the _ pattern but spell out all cases, so that the compiler will
        // bark when a case is missed upon future updates to the types.
        (Type::U8, _)
        | (Type::U16, _)
        | (Type::U32, _)
        | (Type::U64, _)
        | (Type::U128, _)
        | (Type::U256, _)
        | (Type::Bool, _)
        | (Type::Address, _)
        | (Type::Signer, _)
        | (Type::Struct { .. }, _)
        | (Type::StructInstantiation { .. }, _)
        | (Type::Function { .. }, _)
        | (Type::Vector(_), _)
        | (Type::MutableReference(_), _)
        | (Type::Reference(_), _) => false,
    }
}
