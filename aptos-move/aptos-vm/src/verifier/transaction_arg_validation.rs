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
use move_binary_format::file_format_common::read_uleb128_as_u64;
use move_core_types::{account_address::AccountAddress, value::MoveValue, vm_status::StatusCode};
use move_vm_runtime::session::LoadedFunctionInstantiation;
use move_vm_types::loaded_data::runtime_types::Type;
use once_cell::sync::Lazy;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};
use move_core_types::language_storage::ModuleId;
use move_core_types::identifier::Identifier;
use move_core_types::identifier::IdentStr;
use move_vm_types::gas::GasMeter;

pub(crate) struct FunctionId {
    module_id: ModuleId,
    func_name: &'static str,
}

static ALLOWED_STRUCTS: Lazy<BTreeMap<String, FunctionId>> = Lazy::new(|| {
    [("0x1::string::String", FunctionId { module_id: ModuleId::new(AccountAddress::ONE, Identifier::new("string").expect("cannot fail")), func_name: "utf8"}),
        ("0x1::object::Object", FunctionId { module_id: ModuleId::new(AccountAddress::ONE, Identifier::new("object").expect("cannot fail")), func_name: "address_to_object"})]
        .into_iter()
        .map(|(s, validator)| (s.to_string(), validator))
        .collect()
});

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
    mut args: Vec<Vec<u8>>,
    func: &LoadedFunctionInstantiation,
    gas_meter: &mut impl GasMeter
) -> Result<Vec<Vec<u8>>, VMStatus> {
    // entry function should not return
    if !func.return_.is_empty() {
        return Err(VMStatus::Error(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE, None));
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

    if (signer_param_cnt + args.len()) != func.parameters.len() {
        return Err(VMStatus::Error(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH, None));
    }

    // validate all non_signer params
    for (idx, ty) in func.parameters[signer_param_cnt..].iter().enumerate() {
        let (valid, needs_construction) = is_valid_txn_arg(session, ty);
        if !valid {
            return Err(VMStatus::Error(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE, None));
        }
        if needs_construction {
            let mut cursor = Cursor::new(&args[idx][..]);
            let mut new_arg = vec![];
            recursively_construct_arg(session, ty,&mut cursor, gas_meter, &mut new_arg)?;
            args[idx] = new_arg;
            // Check cursor has parsed everything
        }
    }

    // if function doesn't require signer, we reuse txn args
    // if the function require signer, we check senders number same as signers
    // and then combine senders with txn args.
    let combined_args = if signer_param_cnt == 0 {
        args
    } else {
        // the number of txn senders should be the same number of signers
        if senders.len() != signer_param_cnt {
            return Err(VMStatus::Error(
                StatusCode::NUMBER_OF_SIGNER_ARGUMENTS_MISMATCH,
                None
            ));
        }
        senders
            .into_iter()
            .map(|s| MoveValue::Signer(s).simple_serialize().unwrap())
            .chain(args)
            .collect()
    };
    Ok(combined_args)
}

// Return whether the argument is valid/allowed and whether it needs validation.
pub(crate) fn is_valid_txn_arg<S: MoveResolverExt>(
    session: &SessionExt<S>,
    typ: &Type,
) -> (bool, bool) {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match typ {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address => (true, false),
        Vector(inner) => is_valid_txn_arg(session, inner),
        Struct(idx) | StructInstantiation(idx, _) => {
            if let Some(st) = session.get_struct_type(*idx) {
                let full_name = format!("{}::{}", st.module.short_str_lossless(), st.name);
                (ALLOWED_STRUCTS.contains_key(&full_name), true)
            } else {
                (false, false)
            }
        },
        Signer | Reference(_) | MutableReference(_) | TyParam(_) => (false, false),
    }
}

fn validate_and_construct<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    expected_type: &Type,
    constructor: &FunctionId,
    cursor: &mut Cursor<&[u8]>,
    gas_meter: &mut impl GasMeter,
) -> Result<Vec<u8>, VMStatus> {
    let (module, function, instantiation) =
        session.load_function_with_type_arg_inference(&constructor.module_id, IdentStr::new(constructor.func_name).expect(""), expected_type)?;
    let mut args = vec![];
    for param_type in &instantiation.parameters {
        let mut arg = vec![];
        recursively_construct_arg(session, param_type, cursor, gas_meter, &mut arg)?;
        args.push(arg);
    }
    let serialized_result = session.execute_instantiated_function(
        module, function, instantiation,
        args, gas_meter).map_err(|_|VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None))?;
    let mut ret_vals = serialized_result.return_values;
    // We know ret_vals.len() == 1
    Ok(ret_vals.pop().expect("Always a result").0)
}

// A Cursor is used to recursively walk the serialized arg manually and correctly.
pub(crate) fn recursively_construct_arg<S: MoveResolverExt>(
    session: &mut SessionExt<S>,
    ty: &Type,
    cursor: &mut Cursor<&[u8]>,
    gas_meter: &mut impl GasMeter,
    arg: &mut Vec<u8>
) -> Result<(), VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match ty {
        Vector(inner) => {
            // get the vector length and iterate over each element
            let mut len = get_len(cursor)?;
            serialize_uleb128(len, arg);
            while len > 0 {
                recursively_construct_arg(session, inner, cursor, gas_meter, arg)?;
                len -= 1;
            }
        },
        // only strings are validated, and given we are here only if one was present
        // (`is_valid_txn_arg`), this match arm must be for a string
        Struct(idx) | StructInstantiation(idx, _) => {
            // validate the struct value, we use `expect()` because that check was already
            // performed in `is_valid_txn_arg`
            let st = session
                .get_struct_type(*idx).ok_or(VMStatus::Error(StatusCode::ABORT_TYPE_MISMATCH_ERROR, None))?;
            let full_name = format!("{}::{}", st.module.short_str_lossless(), st.name);
            let constructor = ALLOWED_STRUCTS
                .get(&full_name).ok_or(VMStatus::Error(StatusCode::INTERNAL_TYPE_ERROR, None))?;
            arg.append(&mut validate_and_construct(session, ty, constructor, cursor, gas_meter)?);
        },
        Bool => read_n_bytes(1, cursor, arg)?,
        U8 => read_n_bytes(1, cursor, arg)?,
        U16 => read_n_bytes(2, cursor, arg)?,
        U32 => read_n_bytes(4, cursor, arg)?,
        U64 => read_n_bytes(8, cursor, arg)?,
        U128 => read_n_bytes(16, cursor, arg)?,
        U256 => read_n_bytes(32, cursor, arg)?,
        Address => read_n_bytes(32, cursor, arg)?,
        Signer |
        Reference(_) | MutableReference(_) |
        TyParam(_) => return Err(VMStatus::Error(StatusCode::ABORT_TYPE_MISMATCH_ERROR, None)),
    };
    Ok(())
}

// String is a vector of bytes, so both string and vector carry a length in the serialized format.
// Length of vectors in BCS uses uleb128 as a compression format.
fn get_len(cursor: &mut Cursor<&[u8]>) -> Result<usize, VMStatus> {
    match read_uleb128_as_u64(cursor) {
        Err(_) => Err(VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None)),
        Ok(len) => Ok(len as usize),
    }
}

fn serialize_uleb128(mut x: usize, dest: &mut Vec<u8>) {
    while x > 128 {
        dest.push((x | 128) as u8);
        x = x >> 7;
    }
    dest.push(x as u8);
}

fn read_n_bytes(n: usize, src: &mut Cursor<&[u8]>, dest: &mut Vec<u8>) -> Result<(), VMStatus> {
    let len = dest.len();
    dest.resize(len + n, 0);
    src.read_exact(&mut dest[len..]).map_err(|_| VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, None))
}
