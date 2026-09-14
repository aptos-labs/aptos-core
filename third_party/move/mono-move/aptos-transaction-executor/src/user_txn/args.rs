// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Placing a transaction payload's wire-format arguments -- signer addresses
//! and BCS blobs -- onto the payload's parameters.

use crate::errors::{invariant_violation, InvalidArguments, MoveExecutionFailure};
use mono_move_core::{
    intern_type_tag,
    interner::view_module_id,
    types::{
        is_signer_or_signer_immut_ref, view_name, view_type, view_type_list, InternedType, Type,
    },
    ExecutionErrorKind, IntoExecutionError, VMInternalError, VMResult,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{read_uleb128_len, CallBuilder, RuntimeError, StorageReader};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{StructTag, TypeTag},
};

/// The most `Object<T>` values one argument may hold.
const MAX_OBJECTS_PER_ARGUMENT: usize = 32;

/// Why a decoded value of a whitelisted framework type was refused.
#[derive(Debug, thiserror::Error)]
enum ValueRejection {
    #[error("a `String` is not valid UTF-8")]
    MalformedString,
    #[error("no object at {0}")]
    ObjectDoesNotExist(AccountAddress),
    #[error("the object at {0} holds no resource of the argument's type")]
    ObjectLacksResource(AccountAddress),
    #[error("more than {MAX_OBJECTS_PER_ARGUMENT} objects in one argument")]
    TooManyObjects,
}

impl IntoExecutionError for ValueRejection {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::InvalidOperation
    }
}

impl From<&ValueRejection> for InvalidArguments {
    fn from(rejection: &ValueRejection) -> Self {
        match rejection {
            ValueRejection::MalformedString => InvalidArguments::MalformedString,
            ValueRejection::ObjectDoesNotExist(_) => InvalidArguments::ObjectDoesNotExist,
            ValueRejection::ObjectLacksResource(_) => InvalidArguments::ObjectLacksResource,
            ValueRejection::TooManyObjects => InvalidArguments::TooManyObjects,
        }
    }
}

/// Counts the leading signer parameters, rejecting a signer that follows a
/// non-signer parameter.
pub(super) fn leading_signer_params(param_tys: &[InternedType]) -> Result<usize, InvalidArguments> {
    let signer_params = param_tys
        .iter()
        .take_while(|&&ty| is_signer_or_signer_immut_ref(ty))
        .count();
    if param_tys[signer_params..]
        .iter()
        .any(|&ty| is_signer_or_signer_immut_ref(ty))
    {
        return Err(InvalidArguments::SignerAfterArgument);
    }
    Ok(signer_params)
}

/// Fills the call in parameter order: the `signer_params` leading signer
/// parameters from the sender and secondary signers, everything else from the
/// transaction's BCS arguments.
pub(super) fn place_user_txn_args<'a>(
    guard: &ExecutionGuard<'_>,
    call: &mut CallBuilder<'a, '_>,
    signer_params: usize,
    sender: &'a AccountAddress,
    secondary_signers: &'a [AccountAddress],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    // Like AptosVM, check both counts before decoding any argument: a function
    // with signer parameters requires exactly that many signers, while one
    // without ignores them.
    if args.len() != call.param_tys().len() - signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::ArgumentCountMismatch,
        ));
    }
    if signer_params > 0 && 1 + secondary_signers.len() != signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::SignerCountMismatch,
        ));
    }
    // The sender fills the first signer parameter, secondary signers the
    // rest. Placing a signer can only fail on a bug.
    for signer in std::iter::once(sender)
        .chain(secondary_signers)
        .take(signer_params)
    {
        call.signer(signer)
            .map_err(MoveExecutionFailure::RuntimeError)?;
    }
    let object_core = object_core_type(guard)?;
    for arg in args {
        let mut objects = 0;
        // A refused value or a decode failure faults the argument; anything
        // else (a non-decodable parameter type, an exhausted heap) is the VM's.
        call.arg_bcs_with(arg, |storage, ty, bytes| {
            check_framework_value(storage, ty, bytes, object_core, &mut objects)
        })
        .map_err(|err| {
            if let Some(rejection) = err.downcast_ref::<ValueRejection>() {
                MoveExecutionFailure::InvalidArguments(rejection.into())
            } else if err
                .downcast_ref::<RuntimeError>()
                .is_some_and(RuntimeError::is_bcs_decode_error)
            {
                MoveExecutionFailure::InvalidArguments(InvalidArguments::UndecodableArgument)
            } else {
                MoveExecutionFailure::RuntimeError(err)
            }
        })?;
    }
    Ok(())
}

/// The interned `0x1::object::ObjectCore`.
fn object_core_type(guard: &ExecutionGuard<'_>) -> Result<InternedType, MoveExecutionFailure> {
    let tag = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("object").to_owned(),
        name: ident_str!("ObjectCore").to_owned(),
        type_args: vec![],
    }));
    intern_type_tag(&tag, guard)
        .map_err(|err| MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string())))
}

/// Refuses the values AptosVM's argument constructors refuse: a `String` that
/// is not valid UTF-8, and an `Object<T>` whose address holds no `ObjectCore`
/// or no `T`. Every other value passes.
fn check_framework_value(
    storage: &mut StorageReader<'_, '_>,
    ty: InternedType,
    bytes: &[u8],
    object_core: InternedType,
    objects: &mut usize,
) -> VMResult<()> {
    let Type::Nominal {
        module_id,
        name,
        ty_args,
    } = view_type(ty)
    else {
        return Ok(());
    };
    let module_id = view_module_id(*module_id);
    if *module_id.address() != AccountAddress::ONE {
        return Ok(());
    }
    match (view_name(module_id.name()), view_name(*name)) {
        ("string", "String") => {
            // A `String` holds a `vector<u8>`: a length prefix, then the text.
            let mut cursor = 0;
            read_uleb128_len(bytes, &mut cursor).map_err(VMInternalError::new)?;
            if std::str::from_utf8(&bytes[cursor..]).is_err() {
                return Err(VMInternalError::new(ValueRejection::MalformedString));
            }
            Ok(())
        },
        ("object", "Object") => {
            *objects += 1;
            if *objects > MAX_OBJECTS_PER_ARGUMENT {
                return Err(VMInternalError::new(ValueRejection::TooManyObjects));
            }
            let address = AccountAddress::from_bytes(bytes)
                .map_err(|_| invariant_violation("an `Object` value is one address"))?;
            let Some(&resource) = view_type_list(*ty_args).first() else {
                return Err(invariant_violation("`Object` takes one type argument"));
            };
            if !storage.resource_exists(address, object_core)? {
                return Err(VMInternalError::new(ValueRejection::ObjectDoesNotExist(
                    address,
                )));
            }
            if !storage.resource_exists(address, resource)? {
                return Err(VMInternalError::new(ValueRejection::ObjectLacksResource(
                    address,
                )));
            }
            Ok(())
        },
        _ => Ok(()),
    }
}
