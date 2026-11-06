// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Return values in V1's encoding.

use crate::OutputError;
use mono_move_core::types::{view_type, InternedType, Type};
use move_core_types::{account_address::AccountAddress, value::MoveValue};

/// Converts a top-level signer's bare address bytes to V1's signer encoding.
/// Returns `bytes` unchanged for other types.
//
// TODO(correctness): a signer nested in a vector or struct keeps MonoMove's
// bare-address encoding. Re-encode through the value's layout instead of by
// top-level type.
pub fn v1_return_bcs(ty: InternedType, bytes: Vec<u8>) -> Result<Vec<u8>, OutputError> {
    if !matches!(view_type(ty), Type::Signer) {
        return Ok(bytes);
    }
    let address = AccountAddress::from_bytes(&bytes)
        .map_err(|err| OutputError::InvalidSigner(err.to_string()))?;
    MoveValue::Signer(address)
        .simple_serialize()
        .ok_or_else(|| OutputError::InvalidSigner("it does not serialize".to_string()))
}
