// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementation of AWS Nitro Enclave attestation verification.
//!
//! Verifies attestation documents produced by AWS Nitro Enclaves (COSE Sign1 format)
//! by validating the certificate chain and signature against AWS root of trust.

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeResult,
};
use move_core_types::{
    gas_algebra::NumBytes,
    language_storage::{OPTION_NONE_TAG, OPTION_SOME_TAG},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

fn option_some(context: &SafeNativeContext, value: Value) -> SafeNativeResult<Value> {
    let enum_option = context.get_feature_flags().is_enum_option_enabled();
    Ok(if enum_option {
        Value::struct_(Struct::pack_variant(OPTION_SOME_TAG, vec![value]))
    } else {
        Value::struct_(Struct::pack(vec![Value::vector_unchecked(vec![value])?]))
    })
}

fn option_none(context: &SafeNativeContext) -> SafeNativeResult<Value> {
    let enum_option = context.get_feature_flags().is_enum_option_enabled();
    Ok(if enum_option {
        Value::struct_(Struct::pack_variant(OPTION_NONE_TAG, vec![]))
    } else {
        Value::struct_(Struct::pack(vec![Value::vector_unchecked(vec![])?]))
    })
}

/***************************************************************************************************
 * native fun verify_attestation
 *
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 *   Returns true if the attestation document (COSE Sign1 bytes from AWS Nitro Enclave)
 *   is valid: well-formed, signed by the Nitro cert chain, and not expired.
 *   Returns false for invalid, malformed, or expired attestations.
 **************************************************************************************************/
fn native_verify_attestation(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let attestation_bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = AWS_NITRO_VERIFY_ATTESTATION_BASE
        + AWS_NITRO_VERIFY_ATTESTATION_PER_BYTE * NumBytes::new(attestation_bytes.len() as u64);
    context.charge(cost)?;

    let valid = attestation_doc_validation::validate_attestation_doc(&attestation_bytes).is_ok();

    Ok(smallvec![Value::bool(valid)])
}

/***************************************************************************************************
 * native fun verify_and_parse_attestation
 *
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 *   Validates the attestation doc and on success returns Some(AttestationDoc) with
 *   module_id, timestamp, digest, pcrs, certificate, user_data, nonce, public_key.
 **************************************************************************************************/
fn native_verify_and_parse_attestation(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 1);

    let attestation_bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = AWS_NITRO_VERIFY_AND_PARSE_ATTESTATION_BASE
        + AWS_NITRO_VERIFY_AND_PARSE_ATTESTATION_PER_BYTE
            * NumBytes::new(attestation_bytes.len() as u64);
    context.charge(cost)?;

    let opt_doc = match attestation_doc_validation::validate_and_parse_attestation_doc(
        &attestation_bytes,
    ) {
        Ok(doc) => {
            let module_id = Value::vector_u8(doc.module_id.as_bytes().to_vec());
            let timestamp = Value::u64(doc.timestamp);
            let digest = Value::vector_u8(format!("{:?}", doc.digest).into_bytes());
            let pcrs_vec: Vec<Value> = doc
                .pcrs
                .iter()
                .map(|(idx, val)| {
                    Value::struct_(Struct::pack(vec![
                        Value::u8((*idx) as u8),
                        Value::vector_u8(val.to_vec()),
                    ]))
                })
                .collect();
            let pcrs_value = Value::vector_unchecked(pcrs_vec)?;
            let certificate = Value::vector_u8(doc.certificate.to_vec());
            let user_data = match &doc.user_data {
                Some(b) => option_some(context, Value::vector_u8(b.to_vec()))?,
                None => option_none(context)?,
            };
            let nonce = match &doc.nonce {
                Some(b) => option_some(context, Value::vector_u8(b.to_vec()))?,
                None => option_none(context)?,
            };
            let public_key = match &doc.public_key {
                Some(b) => option_some(context, Value::vector_u8(b.to_vec()))?,
                None => option_none(context)?,
            };
            let attestation_doc = Value::struct_(Struct::pack(vec![
                module_id,
                timestamp,
                digest,
                pcrs_value,
                certificate,
                user_data,
                nonce,
                public_key,
            ]));
            option_some(context, attestation_doc)?
        }
        Err(_) => option_none(context)?,
    };

    Ok(smallvec![opt_doc])
}

#[cfg(test)]
mod tests {
    /// Tests the same validation logic used by the native (attestation_doc_validation crate).
    #[test]
    fn test_empty_attestation_invalid() {
        assert!(attestation_doc_validation::validate_attestation_doc(&[]).is_err());
    }

    #[test]
    fn test_garbage_attestation_invalid() {
        let garbage = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        assert!(attestation_doc_validation::validate_attestation_doc(&garbage).is_err());
    }

    /// Happy path: real attestation doc from AWS Nitro Enclave.
    /// Run with: cargo test -p aptos-framework test_real_attestation -- --ignored
    /// (Requires /tmp/attestation-3586.bin to exist.)
    #[test]
    #[ignore = "requires real attestation doc at /tmp/attestation-3586.bin"]
    fn test_real_attestation_valid() {
        let path = std::path::Path::new("/tmp/attestation-3586.bin");
        let bytes = std::fs::read(path).expect("attestation doc at /tmp/attestation-3586.bin");
        assert!(
            attestation_doc_validation::validate_attestation_doc(&bytes).is_ok(),
            "real attestation doc should validate"
        );
    }

    /// Happy path for verify_and_parse: real attestation doc parses and has expected fields.
    /// Run with: cargo test -p aptos-framework test_real_attestation_parse -- --ignored
    #[test]
    #[ignore = "requires real attestation doc at /tmp/attestation-3586.bin"]
    fn test_real_attestation_parse() {
        let path = std::path::Path::new("/tmp/attestation-3586.bin");
        let bytes = std::fs::read(path).expect("attestation doc at /tmp/attestation-3586.bin");
        let doc = attestation_doc_validation::validate_and_parse_attestation_doc(&bytes)
            .expect("real attestation doc should parse");
        assert!(!doc.module_id.is_empty(), "module_id non-empty");
        assert!(!doc.certificate.is_empty(), "certificate non-empty");
        assert!(!doc.pcrs.is_empty(), "pcrs non-empty");
    }
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("verify_attestation", native_verify_attestation as RawSafeNative),
        (
            "verify_and_parse_attestation",
            native_verify_and_parse_attestation as RawSafeNative,
        ),
    ];

    builder.make_named_natives(natives)
}
