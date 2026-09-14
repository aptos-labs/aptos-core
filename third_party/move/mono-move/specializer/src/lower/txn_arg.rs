// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The transaction-argument deserialization intrinsic.
//!
//! The VM-provided `txn_arg` module declares `deserialize<T>` with a body that
//! only aborts. Lowering rewrites each call to it into a call to the module's
//! deserializer for the concrete `T`, so recursing on a type is
//! monomorphization.

use mono_move_core::{
    interner::{
        deserializer_name, generated_deserializers_module_id, is_vm_module_id, txn_arg_module_id,
        view_module_id, InternedIdentifier, InternedModuleId,
    },
    types::{
        type_to_string, view_name, view_type, view_type_list, InternedType, InternedTypeList, Type,
        EMPTY_TYPE_LIST,
    },
    ExecutionErrorKind, Interner, IntoExecutionError, VMInternalError, VMResult,
};
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};
use thiserror::Error;

/// The intrinsic's name in the `txn_arg` module.
pub const DESERIALIZE: &IdentStr = ident_str!("deserialize");

/// A type no transaction argument can have.
#[derive(Debug, Error)]
#[error("`{ty}` is not a transaction argument type")]
pub struct NotATransactionArgument {
    pub ty: String,
}

impl IntoExecutionError for NotATransactionArgument {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::LinkingError
    }
}

/// The call target replacing a call to the intrinsic from inside a VM module,
/// or `None` if the call is to anything else.
pub fn resolve_intrinsic(
    interner: &impl Interner,
    enclosing_module: InternedModuleId,
    callee_module_id: InternedModuleId,
    callee_func_name: InternedIdentifier,
    ty_args: InternedTypeList,
) -> VMResult<Option<(InternedModuleId, InternedIdentifier, InternedTypeList)>> {
    let txn_arg = txn_arg_module_id(interner);
    if !is_vm_module_id(enclosing_module)
        || callee_module_id != txn_arg
        || callee_func_name != interner.identifier_of(DESERIALIZE)
    {
        return Ok(None);
    }
    let &[ty] = view_type_list(ty_args) else {
        return Err(VMInternalError::new(NotATransactionArgument {
            ty: "<malformed intrinsic call>".to_string(),
        }));
    };
    deserializer_for(interner, txn_arg, ty).map(Some)
}

/// The deserializer of `ty`: one of the `txn_arg` module's for the framework
/// types it knows, otherwise the generated one for a public struct or enum,
/// which exists only if the type is eligible.
fn deserializer_for(
    interner: &impl Interner,
    txn_arg: InternedModuleId,
    ty: InternedType,
) -> VMResult<(InternedModuleId, InternedIdentifier, InternedTypeList)> {
    let not_an_argument = || {
        VMInternalError::new(NotATransactionArgument {
            ty: type_to_string(ty),
        })
    };
    let in_txn_arg = |name: &IdentStr, ty_args| (txn_arg, interner.identifier_of(name), ty_args);
    Ok(match view_type(ty) {
        Type::Bool => in_txn_arg(ident_str!("deserialize_bool"), EMPTY_TYPE_LIST),
        Type::U8 => in_txn_arg(ident_str!("deserialize_u8"), EMPTY_TYPE_LIST),
        Type::U16 => in_txn_arg(ident_str!("deserialize_u16"), EMPTY_TYPE_LIST),
        Type::U32 => in_txn_arg(ident_str!("deserialize_u32"), EMPTY_TYPE_LIST),
        Type::U64 => in_txn_arg(ident_str!("deserialize_u64"), EMPTY_TYPE_LIST),
        Type::U128 => in_txn_arg(ident_str!("deserialize_u128"), EMPTY_TYPE_LIST),
        Type::U256 => in_txn_arg(ident_str!("deserialize_u256"), EMPTY_TYPE_LIST),
        Type::Address => in_txn_arg(ident_str!("deserialize_address"), EMPTY_TYPE_LIST),
        Type::Vector { elem } => in_txn_arg(
            ident_str!("deserialize_vector"),
            interner.type_list_of(&[*elem]),
        ),
        Type::Nominal {
            module_id,
            name,
            ty_args,
        } => {
            let module = view_module_id(*module_id);
            let framework = match (
                *module.address() == AccountAddress::ONE,
                view_name(module.name()),
                view_name(*name),
            ) {
                (true, "option", "Option") => Some(ident_str!("deserialize_option")),
                (true, "string", "String") => Some(ident_str!("deserialize_string")),
                (true, "object", "Object") => Some(ident_str!("deserialize_object")),
                (true, "fixed_point32", "FixedPoint32") => {
                    Some(ident_str!("deserialize_fixed_point32"))
                },
                (true, "fixed_point64", "FixedPoint64") => {
                    Some(ident_str!("deserialize_fixed_point64"))
                },
                _ => None,
            };
            match framework {
                Some(deserializer) => in_txn_arg(deserializer, *ty_args),
                None => (
                    generated_deserializers_module_id(interner, *module_id),
                    interner.identifier_of(&deserializer_name(view_name(*name))),
                    *ty_args,
                ),
            }
        },
        // TODO(completeness): signed integers, which AptosVM accepts.
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Signer
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => return Err(not_an_argument()),
    })
}
