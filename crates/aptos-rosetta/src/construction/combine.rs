// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{check_network, decode_bcs, decode_key, encode_bcs},
    error::{ApiError, ApiResult},
    types::*,
    RosettaContext,
};
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_logger::debug;
use aptos_types::transaction::{RawTransaction, SignedTransaction};

/// Construction combine command (OFFLINE)
///
/// This combines signatures, and a raw transaction
///
/// This currently only supports the original Ed25519 with single signer.
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructioncombine)
pub(crate) async fn construction_combine(
    request: ConstructionCombineRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionCombineResponse> {
    debug!("/construction/combine {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Decode the unsigned transaction from BCS in the input
    let unsigned_txn: RawTransaction =
        decode_bcs(&request.unsigned_transaction, "UnsignedTransaction")?;

    // Single signer only supported for now
    // TODO: Support multi-agent / multi-signer?
    if request.signatures.len() != 1 {
        return Err(ApiError::UnsupportedSignatureCount(Some(
            request.signatures.len(),
        )));
    }

    let signature = &request.signatures[0];

    // Only support Ed25519
    if signature.signature_type != SignatureType::Ed25519
        || signature.public_key.curve_type != CurveType::Edwards25519
    {
        return Err(ApiError::InvalidSignatureType);
    }

    // Decode the key and signature accordingly
    let public_key: Ed25519PublicKey =
        decode_key(&signature.public_key.hex_bytes, "Ed25519PublicKey")?;
    let signature: Ed25519Signature = decode_key(&signature.hex_bytes, "Ed25519Signature")?;

    // Combine them into a signed transaction, and encode it as BCS to return
    let signed_txn = SignedTransaction::new(unsigned_txn, public_key, signature);

    Ok(ConstructionCombineResponse {
        signed_transaction: encode_bcs(&signed_txn)?,
    })
}
