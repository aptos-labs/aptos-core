// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Module defines validation of transaction arguments.
//!
//! TODO: we should not only validate the types but also the actual values, e.g.
//! for strings whether they consist of correct characters.

use crate::{
    move_vm_ext::{MoveResolverExt, SessionExt},
    VMStatus,
};
use move_binary_format::{errors::VMError, file_format_common::read_uleb128_as_u64};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    value::MoveValue,
    vm_status::StatusCode,
};
use move_vm_runtime::session::LoadedFunctionInstantiation;
use move_vm_types::{
    gas::{GasMeter, UnmeteredGasMeter},
    loaded_data::runtime_types::Type,
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
pub(crate) fn validate_combine_signer_and_txn_args<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    senders: Vec<AccountAddress>,
    args: Vec<Vec<u8>>,
    func: &LoadedFunctionInstantiation,
    are_struct_constructors_enabled: bool,
) -> Result<Vec<Vec<u8>>, VMStatus> {
    // entry function should not return
    if !func.return_.is_empty() {
        return Err(VMStatus::Error(
            StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
            None,
        ));
    }
    let mut signer_param_cnt = 0;
    // find all signer params at the beginning
    for ty in func.parameters.iter() {
        match ty {
            Type::Signer => signer_param_cnt += 1,
            Type::Reference(inner_type) => {
                if matches!(&**inner_type, Type::Signer) {
                    signer_param_cnt += 1;
                }
            },
            _ => (),
        }
    }

    let allowed_structs = get_allowed_structs(are_struct_constructors_enabled);
    // validate all non_signer params
    let mut needs_construction = vec![];
    for (idx, ty) in func.parameters[signer_param_cnt..].iter().enumerate() {
        let (valid, construction) = is_valid_txn_arg(
            session,
            &ty.subst(&func.type_arguments).unwrap(),
            allowed_structs,
        );
        if !valid {
            return Err(VMStatus::Error(
                StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE,
                None,
            ));
        }
        if construction {
            needs_construction.push(idx + signer_param_cnt);
        }
    }

    if (signer_param_cnt + args.len()) != func.parameters.len() {
        return Err(VMStatus::Error(
            StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH,
            None,
        ));
    }
    // if function doesn't require signer, we reuse txn args
    // if the function require signer, we check senders number same as signers
    // and then combine senders with txn args.
    let mut combined_args = if signer_param_cnt == 0 {
        args
    } else {
        // the number of txn senders should be the same number of signers
        if senders.len() != signer_param_cnt {
            return Err(VMStatus::Error(
                StatusCode::NUMBER_OF_SIGNER_ARGUMENTS_MISMATCH,
                None,
            ));
        }
        senders
            .into_iter()
            .map(|s| MoveValue::Signer(s).simple_serialize().unwrap())
            .chain(args)
            .collect()
    };
    if !needs_construction.is_empty() {
        construct_args(
            session,
            &needs_construction,
            &mut combined_args,
            func,
            allowed_structs,
        )?;
    }
    Ok(combined_args)
}

// Return whether the argument is valid/allowed and whether it needs construction.
pub(crate) fn is_valid_txn_arg<S: MoveResolverExt>(
    session: &SessionExt<S>,
    typ: &Type,
    allowed_structs: &ConstructorMap,
) -> (bool, bool) {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match typ {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address => (true, false),
        Vector(inner) => is_valid_txn_arg(session, inner, allowed_structs),
        Struct(idx) | StructInstantiation(idx, _) => {
            if let Some(st) = session.get_struct_type(*idx) {
                let full_name = format!("{}::{}", st.module.short_str_lossless(), st.name);
                (allowed_structs.contains_key(&full_name), true)
            } else {
                (false, false)
            }
        },
        Signer | Reference(_) | MutableReference(_) | TyParam(_) => (false, false),
    }
}

// Construct arguments. Walk through the arguments and according to the signature
// construct arguments that require so.
// TODO: This needs a more solid story and a tighter integration with the VM.
pub(crate) fn construct_args<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    idxs: &[usize],
    args: &mut [Vec<u8>],
    func: &LoadedFunctionInstantiation,
    allowed_structs: &ConstructorMap,
) -> Result<(), VMStatus> {
    // Perhaps in a future we should do proper gas metering here
    let mut gas_meter = UnmeteredGasMeter;
    for (idx, ty) in func.parameters.iter().enumerate() {
        if !idxs.contains(&idx) {
            continue;
        }
        let arg = &mut args[idx];
        let mut cursor = Cursor::new(&arg[..]);
        let mut new_arg = vec![];
        recursively_construct_arg(
            session,
            &ty.subst(&func.type_arguments).unwrap(),
            allowed_structs,
            &mut cursor,
            &mut gas_meter,
            &mut new_arg,
        )?;
        // Check cursor has parsed everything
        // Unfortunately, is_empty is only enabled in nightly, so we check this way.
        if cursor.position() != arg.len() as u64 {
            return Err(VMStatus::Error(
                StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
                Some(String::from(
                    "The serialized arguments to constructor contained extra data",
                )),
            ));
        }
        *arg = new_arg;
    }
    Ok(())
}

// A Cursor is used to recursively walk the serialized arg manually and correctly. In effect we
// are parsing the BCS serialized implicit constructor invocation tree, while serializing the
// constructed types into the output parameter arg.
pub(crate) fn recursively_construct_arg<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    ty: &Type,
    allowed_structs: &ConstructorMap,
    cursor: &mut Cursor<&[u8]>,
    gas_meter: &mut impl GasMeter,
    arg: &mut Vec<u8>,
) -> Result<(), VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match ty {
        Vector(inner) => {
            // get the vector length and iterate over each element
            let mut len = get_len(cursor)?;
            serialize_uleb128(len, arg);
            while len > 0 {
                recursively_construct_arg(session, inner, allowed_structs, cursor, gas_meter, arg)?;
                len -= 1;
            }
        },
        Struct(idx) | StructInstantiation(idx, _) => {
            // validate the struct value, we use `expect()` because that check was already
            // performed in `is_valid_txn_arg`
            let st = session
                .get_struct_type(*idx)
                .expect("unreachable, type must exist");
            let full_name = format!("{}::{}", st.module.short_str_lossless(), st.name);
            let constructor = allowed_structs
                .get(&full_name)
                .expect("unreachable: struct must be allowed");
            // By appending the BCS to the output parameter we construct the correct BCS format
            // of the argument.
            arg.append(&mut validate_and_construct(
                session,
                ty,
                constructor,
                allowed_structs,
                cursor,
                gas_meter,
            )?);
        },
        Bool | U8 => read_n_bytes(1, cursor, arg)?,
        U16 => read_n_bytes(2, cursor, arg)?,
        U32 => read_n_bytes(4, cursor, arg)?,
        U64 => read_n_bytes(8, cursor, arg)?,
        U128 => read_n_bytes(16, cursor, arg)?,
        U256 | Address => read_n_bytes(32, cursor, arg)?,
        Signer | Reference(_) | MutableReference(_) | TyParam(_) => {
            unreachable!("We already checked for this in is-valid-txn-arg")
        },
    };

    Ok(())
}

// A move function that constructs a type will return the BCS serialized representation of the
// constructed value. This is the correct data to pass as the argument to a function taking
// said struct as a parameter. In this function we execute the constructor constructing the
// value and returning the BCS serialized representation.
fn validate_and_construct<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    expected_type: &Type,
    constructor: &FunctionId,
    allowed_structs: &ConstructorMap,
    cursor: &mut Cursor<&[u8]>,
    gas_meter: &mut impl GasMeter,
) -> Result<Vec<u8>, VMStatus> {
    let (function, instantiation) = session.load_function_with_type_arg_inference(
        &constructor.module_id,
        constructor.func_name,
        expected_type,
    )?;
    let mut args = vec![];
    for param_type in &instantiation.parameters {
        let mut arg = vec![];
        recursively_construct_arg(
            session,
            &param_type.subst(&instantiation.type_arguments).unwrap(),
            allowed_structs,
            cursor,
            gas_meter,
            &mut arg,
        )?;
        args.push(arg);
    }
    let constructor_error = |e: VMError| {
        // A slight hack, to prevent additional piping of the feature flag through all
        // function calls. We know the feature is active when more structs then just strings are
        // allowed.
        let are_struct_constructors_enabled = allowed_structs.len() > 1;
        if are_struct_constructors_enabled {
            e.into_vm_status()
        } else {
            VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None)
        }
    };
    let serialized_result = session
        .execute_instantiated_function(function, instantiation, args, gas_meter)
        .map_err(constructor_error)?;
    let mut ret_vals = serialized_result.return_values;
    // We know ret_vals.len() == 1
    let deserialize_error = VMStatus::Error(
        StatusCode::INTERNAL_TYPE_ERROR,
        Some(String::from("Constructor did not return value")),
    );
    Ok(ret_vals.pop().ok_or(deserialize_error)?.0)
}

// String is a vector of bytes, so both string and vector carry a length in the serialized format.
// Length of vectors in BCS uses uleb128 as a compression format.
fn get_len(cursor: &mut Cursor<&[u8]>) -> Result<usize, VMStatus> {
    match read_uleb128_as_u64(cursor) {
        Err(_) => Err(VMStatus::Error(
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
    let len = dest.len();
    dest.resize(len + n, 0);
    src.read_exact(&mut dest[len..]).map_err(|_| {
        VMStatus::Error(
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            Some(String::from("Couldn't read bytes")),
        )
    })
}
