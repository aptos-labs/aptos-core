// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Module defines validation of transaction arguments.
//!
//! TODO: we should not only validate the types but also the actual values, e.g.
//! for strings whether they consist of correct characters.

use crate::{
    aptos_vm::SerializedSigners,
    move_vm_ext::{AptosMoveResolver, SessionExt},
    VMStatus,
};
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    file_format::FunctionDefinitionIndex,
    file_format_common::read_uleb128_as_u64,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_runtime::{
    module_traversal::{TraversalContext, TraversalStorage},
    LoadedFunction, LoadedFunctionOwner, ModuleStorage, RuntimeEnvironment,
};
use move_vm_types::{
    gas::{GasMeter, UnmeteredGasMeter},
    loaded_data::runtime_types::{Type, TypeParamMap},
};
use once_cell::sync::Lazy;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};

pub(crate) struct FunctionId {
    module_id: ModuleId,
    func_name: &'static IdentStr,
}

type ConstructorMap = Lazy<BTreeMap<String, FunctionId>>;
static OLD_ALLOWED_STRUCTS: ConstructorMap = Lazy::new(|| {
    [("0x1::string::String", FunctionId {
        module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("string"))),
        func_name: ident_str!("utf8"),
    })]
    .into_iter()
    .map(|(s, validator)| (s.to_string(), validator))
    .collect()
});

static NEW_ALLOWED_STRUCTS: ConstructorMap = Lazy::new(|| {
    [
        ("0x1::string::String", FunctionId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("string"))),
            func_name: ident_str!("utf8"),
        }),
        ("0x1::object::Object", FunctionId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("object"))),
            func_name: ident_str!("address_to_object"),
        }),
        ("0x1::option::Option", FunctionId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::from(ident_str!("option"))),
            func_name: ident_str!("from_vec"),
        }),
        ("0x1::fixed_point32::FixedPoint32", FunctionId {
            module_id: ModuleId::new(
                AccountAddress::ONE,
                Identifier::from(ident_str!("fixed_point32")),
            ),
            func_name: ident_str!("create_from_raw_value"),
        }),
        ("0x1::fixed_point64::FixedPoint64", FunctionId {
            module_id: ModuleId::new(
                AccountAddress::ONE,
                Identifier::from(ident_str!("fixed_point64")),
            ),
            func_name: ident_str!("create_from_raw_value"),
        }),
    ]
    .into_iter()
    .map(|(s, validator)| (s.to_string(), validator))
    .collect()
});

pub(crate) fn get_allowed_structs(
    are_struct_constructors_enabled: bool,
) -> &'static ConstructorMap {
    if are_struct_constructors_enabled {
        &NEW_ALLOWED_STRUCTS
    } else {
        &OLD_ALLOWED_STRUCTS
    }
}

/// Validate and generate args for entry function
/// validation includes:
/// 1. return signature is empty
/// 2. number of signers is same as the number of senders
/// 3. check arg types are allowed after signers
///
/// after validation, add senders and non-signer arguments to generate the final args
pub(crate) fn validate_combine_signer_and_txn_args(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    serialized_signers: &SerializedSigners,
    args: Vec<Vec<u8>>,
    func: &LoadedFunction,
    are_struct_constructors_enabled: bool,
) -> Result<Vec<Vec<u8>>, VMStatus> {
    let _timer = VM_TIMER.timer_with_label("AptosVM::validate_combine_signer_and_txn_args");

    // Entry function should not return.
    if !func.return_tys().is_empty() {
        return Err(VMStatus::error(
            StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
            None,
        ));
    }
    let mut signer_param_cnt = 0;
    // find all signer params at the beginning
    for ty in func.param_tys() {
        if ty.is_signer_or_signer_ref() {
            signer_param_cnt += 1;
        }
    }

    let allowed_structs = get_allowed_structs(are_struct_constructors_enabled);
    let ty_builder = &module_storage.runtime_environment().vm_config().ty_builder;

    // Need to keep this here to ensure we return the historic correct error code for replay
    for ty in func.param_tys()[signer_param_cnt..].iter() {
        let subst_res = ty_builder.create_ty_with_subst(ty, func.ty_args());
        let ty = subst_res.map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
        let valid = is_valid_txn_arg(module_storage.runtime_environment(), &ty, allowed_structs);
        if !valid {
            return Err(VMStatus::error(
                StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
                None,
            ));
        }
    }

    if (signer_param_cnt + args.len()) != func.param_tys().len() {
        return Err(VMStatus::error(
            StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH,
            None,
        ));
    }

    // If the invoked function expects one or more signers, we need to check that the number of
    // signers actually passed is matching first to maintain backward compatibility before
    // moving on to the validation of non-signer args.
    // the number of txn senders should be the same number of signers
    let sender_signers = serialized_signers.senders();
    if signer_param_cnt > 0 && sender_signers.len() != signer_param_cnt {
        return Err(VMStatus::error(
            StatusCode::NUMBER_OF_SIGNER_ARGUMENTS_MISMATCH,
            None,
        ));
    }

    // This also validates that the args are valid. If they are structs, they have to be allowed
    // and must be constructed successfully. If construction fails, this would fail with a
    // FAILED_TO_DESERIALIZE_ARGUMENT error.
    let args = construct_args(
        session,
        module_storage,
        &func.param_tys()[signer_param_cnt..],
        args,
        func.ty_args(),
        allowed_structs,
        false,
    )?;

    // Combine signer and non-signer arguments.
    let combined_args = if signer_param_cnt == 0 {
        args
    } else {
        sender_signers.into_iter().chain(args).collect()
    };
    Ok(combined_args)
}

/// Returns true if the argument is valid (that is, it is a primitive type or a struct with a
/// known constructor function). Otherwise, (for structs without constructors, signers or
/// references) returns false. An error is returned in cases when a struct type is encountered and
/// its name cannot be queried for some reason.
pub(crate) fn is_valid_txn_arg(
    runtime_environment: &RuntimeEnvironment,
    ty: &Type,
    allowed_structs: &ConstructorMap,
) -> bool {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match ty {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address => true,
        Vector(inner) => is_valid_txn_arg(runtime_environment, inner, allowed_structs),
        Struct { .. } | StructInstantiation { .. } => {
            // Note: Original behavior was to return false even if the module loading fails (e.g.,
            //       if struct does not exist. This preserves it.
            runtime_environment
                .get_struct_name(ty)
                .ok()
                .flatten()
                .is_some_and(|(module_id, identifier)| {
                    allowed_structs.contains_key(&format!(
                        "{}::{}",
                        module_id.short_str_lossless(),
                        identifier
                    ))
                })
        },
        Signer | Reference(_) | MutableReference(_) | TyParam(_) | Function { .. } => false,
    }
}

// Construct arguments. Walk through the arguments and according to the signature
// construct arguments that require so.
// TODO: This needs a more solid story and a tighter integration with the VM.
pub(crate) fn construct_args(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    types: &[Type],
    args: Vec<Vec<u8>>,
    ty_args: &[Type],
    allowed_structs: &ConstructorMap,
    is_view: bool,
) -> Result<Vec<Vec<u8>>, VMStatus> {
    // Perhaps in a future we should do proper gas metering here
    let mut gas_meter = UnmeteredGasMeter;
    let mut res_args = vec![];
    if types.len() != args.len() {
        return Err(invalid_signature());
    }

    let ty_builder = &module_storage.runtime_environment().vm_config().ty_builder;
    for (ty, arg) in types.iter().zip(args) {
        let subst_res = ty_builder.create_ty_with_subst(ty, ty_args);
        let ty = subst_res.map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
        let arg = construct_arg(
            session,
            module_storage,
            &ty,
            allowed_structs,
            arg,
            &mut gas_meter,
            is_view,
        )?;
        res_args.push(arg);
    }
    Ok(res_args)
}

fn invalid_signature() -> VMStatus {
    VMStatus::error(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE, None)
}

fn construct_arg(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    ty: &Type,
    allowed_structs: &ConstructorMap,
    arg: Vec<u8>,
    gas_meter: &mut impl GasMeter,
    is_view: bool,
) -> Result<Vec<u8>, VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;
    match ty {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address => Ok(arg),
        Vector(_) | Struct { .. } | StructInstantiation { .. } => {
            let initial_cursor_len = arg.len();
            let mut cursor = Cursor::new(&arg[..]);
            let mut new_arg = vec![];
            let mut max_invocations = 10; // Read from config in the future
            recursively_construct_arg(
                session,
                module_storage,
                ty,
                allowed_structs,
                &mut cursor,
                initial_cursor_len,
                gas_meter,
                &mut max_invocations,
                &mut new_arg,
            )?;
            // Check cursor has parsed everything
            // Unfortunately, is_empty is only enabled in nightly, so we check this way.
            if cursor.position() != initial_cursor_len as u64 {
                return Err(VMStatus::error(
                    StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                    Some(String::from(
                        "The serialized arguments to constructor contained extra data",
                    )),
                ));
            }
            Ok(new_arg)
        },
        Signer => {
            if is_view {
                Ok(arg)
            } else {
                Err(invalid_signature())
            }
        },
        Reference(_) | MutableReference(_) | TyParam(_) | Function { .. } => {
            Err(invalid_signature())
        },
    }
}

// A Cursor is used to recursively walk the serialized arg manually and correctly. In effect we
// are parsing the BCS serialized implicit constructor invocation tree, while serializing the
// constructed types into the output parameter arg.
pub(crate) fn recursively_construct_arg(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    ty: &Type,
    allowed_structs: &ConstructorMap,
    cursor: &mut Cursor<&[u8]>,
    initial_cursor_len: usize,
    gas_meter: &mut impl GasMeter,
    max_invocations: &mut u64,
    arg: &mut Vec<u8>,
) -> Result<(), VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match ty {
        Vector(inner) => {
            // get the vector length and iterate over each element
            let mut len = get_len(cursor)?;
            serialize_uleb128(len, arg);
            while len > 0 {
                recursively_construct_arg(
                    session,
                    module_storage,
                    inner,
                    allowed_structs,
                    cursor,
                    initial_cursor_len,
                    gas_meter,
                    max_invocations,
                    arg,
                )?;
                len -= 1;
            }
        },
        Struct { .. } | StructInstantiation { .. } => {
            let (module_id, identifier) = module_storage
                .runtime_environment()
                .get_struct_name(ty)
                .map_err(|_| {
                    // Note: The original behaviour was to map all errors to an invalid signature
                    //       error, here we want to preserve it for now.
                    invalid_signature()
                })?
                .ok_or_else(invalid_signature)?;
            let full_name = format!("{}::{}", module_id.short_str_lossless(), identifier);
            let constructor = allowed_structs
                .get(&full_name)
                .ok_or_else(invalid_signature)?;
            // By appending the BCS to the output parameter we construct the correct BCS format
            // of the argument.
            arg.append(&mut validate_and_construct(
                session,
                module_storage,
                ty,
                constructor,
                allowed_structs,
                cursor,
                initial_cursor_len,
                gas_meter,
                max_invocations,
            )?);
        },
        Bool | U8 => read_n_bytes(1, cursor, arg)?,
        U16 => read_n_bytes(2, cursor, arg)?,
        U32 => read_n_bytes(4, cursor, arg)?,
        U64 => read_n_bytes(8, cursor, arg)?,
        U128 => read_n_bytes(16, cursor, arg)?,
        U256 | Address => read_n_bytes(32, cursor, arg)?,
        Signer | Reference(_) | MutableReference(_) | TyParam(_) | Function { .. } => {
            return Err(invalid_signature())
        },
    };
    Ok(())
}

// A move function that constructs a type will return the BCS serialized representation of the
// constructed value. This is the correct data to pass as the argument to a function taking
// said struct as a parameter. In this function we execute the constructor constructing the
// value and returning the BCS serialized representation.
fn validate_and_construct(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    expected_type: &Type,
    constructor: &FunctionId,
    allowed_structs: &ConstructorMap,
    cursor: &mut Cursor<&[u8]>,
    initial_cursor_len: usize,
    gas_meter: &mut impl GasMeter,
    max_invocations: &mut u64,
) -> Result<Vec<u8>, VMStatus> {
    if *max_invocations == 0 {
        return Err(VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            None,
        ));
    }
    // HACK mitigation of performance attack
    // To maintain compatibility with vector<string> or so on, we need to allow unlimited strings.
    // So we do not count the string constructor against the max_invocations, instead we
    // shortcut the string case to avoid the performance attack.
    if constructor.func_name.as_str() == "utf8" {
        let constructor_error = || {
            // A slight hack, to prevent additional piping of the feature flag through all
            // function calls. We know the feature is active when more structs then just strings are
            // allowed.
            let are_struct_constructors_enabled = allowed_structs.len() > 1;
            if are_struct_constructors_enabled {
                PartialVMError::new(StatusCode::ABORTED)
                    .with_sub_status(1)
                    .at_code_offset(FunctionDefinitionIndex::new(0), 0)
                    .finish(Location::Module(constructor.module_id.clone()))
                    .into_vm_status()
            } else {
                VMStatus::error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None)
            }
        };
        // Short cut for the utf8 constructor, which is a special case.
        let len = get_len(cursor)?;
        if !cursor
            .position()
            .checked_add(len as u64)
            .is_some_and(|l| l <= initial_cursor_len as u64)
        {
            // We need to make sure we do not allocate more bytes than
            // needed.
            return Err(VMStatus::error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some("String argument is too long".to_string()),
            ));
        }

        let mut arg = vec![];
        read_n_bytes(len, cursor, &mut arg)?;
        std::str::from_utf8(&arg).map_err(|_| constructor_error())?;
        return bcs::to_bytes(&arg)
            .map_err(|_| VMStatus::error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None));
    } else {
        *max_invocations -= 1;
    }

    let function = load_constructor_function(
        module_storage,
        &constructor.module_id,
        constructor.func_name,
        expected_type,
    )?;
    let mut args = vec![];
    let ty_builder = &module_storage.runtime_environment().vm_config().ty_builder;
    for param_ty in function.param_tys() {
        let mut arg = vec![];
        let arg_ty = ty_builder
            .create_ty_with_subst(param_ty, function.ty_args())
            .unwrap();

        recursively_construct_arg(
            session,
            module_storage,
            &arg_ty,
            allowed_structs,
            cursor,
            initial_cursor_len,
            gas_meter,
            max_invocations,
            &mut arg,
        )?;
        args.push(arg);
    }
    let storage = TraversalStorage::new();
    let serialized_result = session.execute_loaded_function(
        function,
        args,
        gas_meter,
        &mut TraversalContext::new(&storage),
        module_storage,
    )?;
    let mut ret_vals = serialized_result.return_values;
    // We know ret_vals.len() == 1
    Ok(ret_vals
        .pop()
        .ok_or_else(|| {
            VMStatus::error(
                StatusCode::INTERNAL_TYPE_ERROR,
                Some(String::from("Constructor did not return value")),
            )
        })?
        .0)
}

// String is a vector of bytes, so both string and vector carry a length in the serialized format.
// Length of vectors in BCS uses uleb128 as a compression format.
fn get_len(cursor: &mut Cursor<&[u8]>) -> Result<usize, VMStatus> {
    match read_uleb128_as_u64(cursor) {
        Err(_) => Err(VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            None,
        )),
        Ok(len) => Ok(len as usize),
    }
}

fn serialize_uleb128(mut x: usize, dest: &mut Vec<u8>) {
    // TODO perhaps reuse the code from move_binary_format::file_format_common if it's public
    while x >= 128 {
        dest.push((x | 128) as u8);
        x >>= 7;
    }
    dest.push(x as u8);
}

fn read_n_bytes(n: usize, src: &mut Cursor<&[u8]>, dest: &mut Vec<u8>) -> Result<(), VMStatus> {
    let deserialization_error = |msg: &str| -> VMStatus {
        VMStatus::error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            Some(msg.to_string()),
        )
    };
    let len = dest.len();

    // It is safer to limit the length under some big (but still reasonable
    // number).
    const MAX_NUM_BYTES: usize = 1_000_000;
    if !len.checked_add(n).is_some_and(|s| s <= MAX_NUM_BYTES) {
        return Err(deserialization_error(&format!(
            "Couldn't read bytes: maximum limit of {} bytes exceeded",
            MAX_NUM_BYTES
        )));
    }

    // Ensure we have enough capacity for resizing.
    dest.try_reserve(len + n)
        .map_err(|e| deserialization_error(&format!("Couldn't read bytes: {}", e)))?;
    dest.resize(len + n, 0);
    src.read_exact(&mut dest[len..])
        .map_err(|_| deserialization_error("Couldn't read bytes"))
}

fn load_constructor_function(
    module_storage: &impl ModuleStorage,
    module_id: &ModuleId,
    function_name: &IdentStr,
    expected_return_ty: &Type,
) -> VMResult<LoadedFunction> {
    // INVARIANT:
    //   We do not need to charge gas for module loading due to invoking a constructor function,
    //   i.e., a function like `utf8()`, or `address_to_object()`. This is safe to do because all
    //   constructor functions are located at 0x1 (special address) and so should not be charged.
    if !module_id.address().is_special() {
        let msg = format!(
            "Constructor function {}::{}::{} has a non-special address!",
            module_id.address(),
            module_id.name(),
            function_name
        );
        let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
            .with_message(msg)
            .finish(Location::Undefined);
        return Err(err);
    }
    let (module, function) = module_storage.unmetered_get_function_definition(
        module_id.address(),
        module_id.name(),
        function_name,
    )?;

    if function.return_tys().len() != 1 {
        // For functions that are marked constructor this should not happen.
        return Err(PartialVMError::new(StatusCode::ABORTED).finish(Location::Undefined));
    }

    let mut map = TypeParamMap::default();
    if !map.match_ty(&function.return_tys()[0], expected_return_ty) {
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
        ty_args.push(map.get_ty_param(i as u16).ok_or_else(|| {
            PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                .finish(Location::Undefined)
        })?);
    }

    Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
        .map_err(|e| e.finish(Location::Module(module_id.clone())))?;

    Ok(LoadedFunction {
        owner: LoadedFunctionOwner::Module(module),
        ty_args,
        function,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(true)]
    #[test_case(false)]
    fn test_constructor_functions_always_have_special_address(
        are_struct_constructors_enabled: bool,
    ) {
        let constructors = get_allowed_structs(are_struct_constructors_enabled);
        for function_id in constructors.values() {
            assert!(function_id.module_id.address().is_special());
        }
    }
}
