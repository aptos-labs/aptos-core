// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{check_network, decode_key},
    error::ApiResult,
    types::*,
    RosettaContext,
};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_logger::debug;
use aptos_types::transaction::authenticator::AuthenticationKey;

/// Construction derive command (OFFLINE)
///
/// Derive account address from Public key
/// Note: This only works for new accounts.  After the account is created, all APIs should provide
/// both account and key.
///
/// Note: if the accounts are handled ONLY by Rosetta, then this will always work.  It only stops working
/// if it is one of many other types of keys / a rotated account.
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionderive)
pub(crate) async fn construction_derive(
    request: ConstructionDeriveRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionDeriveResponse> {
    debug!("/construction/derive {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // The input must be an Ed25519 Public key and will only derive the Address for the original
    // Aptos Ed25519 authentication scheme
    let public_key: Ed25519PublicKey =
        decode_key(&request.public_key.hex_bytes, "Ed25519PublicKey")?;
    let address = AuthenticationKey::ed25519(&public_key).account_address();

    Ok(ConstructionDeriveResponse {
        account_identifier: AccountIdentifier::base_account(address),
    })
}
