// Copyright (c) Aptos
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
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

// A map which contains the structs allowed as transaction input and the
// validation function for those, if one was needed (None otherwise).
// The validation function takes the serialized argument and returns
// an error if the validation fails.
type ValidateArg = fn(&[u8]) -> Result<(), VMStatus>;

static ALLOWED_STRUCTS: Lazy<BTreeMap<String, Option<ValidateArg>>> = Lazy::new(|| {
    [("0x1::string::String", Some(check_string as ValidateArg))]
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
    session: &SessionExt<S>,
    senders: Vec<AccountAddress>,
    args: Vec<Vec<u8>>,
    func: &LoadedFunctionInstantiation,
) -> Result<Vec<Vec<u8>>, VMStatus> {
    // entry function should not return
    if !func.return_.is_empty() {
        return Err(VMStatus::Error(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE));
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
            }
            _ => (),
        }
    }
    // validate all non_signer params
    let mut needs_validation = vec![];
    for (idx, ty) in func.parameters[signer_param_cnt..].iter().enumerate() {
        let (valid, validation) = is_valid_txn_arg(session, ty);
        if !valid {
            return Err(VMStatus::Error(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE));
        }
        if validation {
            needs_validation.push(idx + signer_param_cnt);
        }
    }

    if (signer_param_cnt + args.len()) != func.parameters.len() {
        return Err(VMStatus::Error(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH));
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
            ));
        }
        senders
            .into_iter()
            .map(|s| MoveValue::Signer(s).simple_serialize().unwrap())
            .chain(args)
            .collect()
    };
    if !needs_validation.is_empty() {
        validate_args(session, &needs_validation, &combined_args, func)?;
    }
    Ok(combined_args)
}

// Return whether the argument is valid/allowed and whether it needs validation.
// Validation is only needed for String arguments at the moment and vectors of them.
fn is_valid_txn_arg<S: MoveResolverExt>(session: &SessionExt<S>, typ: &Type) -> (bool, bool) {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    match typ {
        Bool | U8 | U64 | U128 | Address => (true, false),
        Vector(inner) => is_valid_txn_arg(session, inner),
        Struct(idx) | StructInstantiation(idx, _) => {
            if let Some(st) = session.get_struct_type(*idx) {
                let full_name = format!("{}::{}", st.module.short_str_lossless(), st.name);
                match ALLOWED_STRUCTS.get(&full_name) {
                    None => (false, false),
                    Some(validator) => (true, validator.is_some()),
                }
            } else {
                (false, false)
            }
        }
        Signer | Reference(_) | MutableReference(_) | TyParam(_) => (false, false),
    }
}

// Validate arguments. Walk through the arguments and according to the signature
// validate arguments that require so.
// TODO: This needs a more solid story and a tighter integration with the VM.
// Validation at the moment is only for Strings and Vector of them, so we
// manually walk the serialized args until we find a string.
// This is obviously brittle and something to change at some point soon.
fn validate_args<S: MoveResolverExt>(
    session: &SessionExt<S>,
    idxs: &[usize],
    args: &[Vec<u8>],
    func: &LoadedFunctionInstantiation,
) -> Result<(), VMStatus> {
    for (idx, (ty, arg)) in func.parameters.iter().zip(args.iter()).enumerate() {
        if !idxs.contains(&idx) {
            continue;
        }
        let arg_len = arg.len();
        let mut cursor = Cursor::new(&arg[..]);
        validate_arg(session, ty, &mut cursor, arg_len)?;
    }
    Ok(())
}

// Validate a single arg. A Cursor is used to walk the serialized arg manually and correctly.
// Only Strings and nested vector of them are validated.
fn validate_arg<S: MoveResolverExt>(
    session: &SessionExt<S>,
    ty: &Type,
    cursor: &mut Cursor<&[u8]>,
    arg_len: usize,
) -> Result<(), VMStatus> {
    use move_vm_types::loaded_data::runtime_types::Type::*;

    Ok(match ty {
        Vector(inner) => {
            // get the vector length and iterate over each element
            let mut len = get_len(cursor)?;
            while len > 0 {
                validate_arg(session, inner, cursor, arg_len)?;
                len -= 1;
            }
        }
        // only strings are validated, and given we are here only if one was present
        // (`is_valid_txn_arg`), this match arm must be for a string
        Struct(idx) | StructInstantiation(idx, _) => {
            // load the struct name, we use `expect()` because that check was already
            // performed in `is_valid_txn_arg`
            let len = get_len(cursor)?;
            let current_pos = cursor.position() as usize;
            match current_pos.checked_add(len) {
                Some(size) => {
                    if size > arg_len {
                        return Err(VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT));
                    }
                }
                None => return Err(VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)),
            }
            // load the serialized string
            let mut s = vec![0u8; len];
            cursor
                .read_exact(&mut s)
                .map_err(|_| VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT))?;
            // validate the struct value, we use `expect()` because that check was already
            // performed in `is_valid_txn_arg`
            let st = session
                .get_struct_type(*idx)
                .expect("unreachable, type must exist");
            let full_name = format!("{}::{}", st.module.short_str_lossless(), st.name);
            let option_validator = ALLOWED_STRUCTS
                .get(&full_name)
                .expect("unreachable: struct must be allowed");
            if let Some(validator) = option_validator {
                validator(&s)?;
            }
        }
        // this is unreachable given the check in `is_valid_txn_arg` and the
        // fact we collect all arguments that involve strings and we validate
        // them and them only
        Bool | U8 | U64 | U128 | Address | Signer | Reference(_) | MutableReference(_)
        | TyParam(_) => unreachable!("Validation is only for arguments with String"),
    })
}

// String is a vector of bytes, so both string and vector carry a length in the serialized format.
// Length of vectors in BCS uses uleb128 as a compression format.
fn get_len(cursor: &mut Cursor<&[u8]>) -> Result<usize, VMStatus> {
    match read_uleb128_as_u64(cursor) {
        Err(_) => Err(VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)),
        Ok(len) => Ok(len as usize),
    }
}

//
// Argument validation functions
//

// Check if a string is valid. This code is copied from string.rs in the stdlib.
// TODO: change the move VM code (string.rs) to expose a function that does validation.
fn check_string(s: &[u8]) -> Result<(), VMStatus> {
    match std::str::from_utf8(s) {
        Ok(_) => Ok(()),
        Err(_) => Err(VMStatus::Error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)),
    }
}
