// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{check_network, decode_bcs},
    error::ApiResult,
    types::*,
    RosettaContext,
};
use aptos_logger::debug;
use aptos_types::transaction::SignedTransaction;

/// Construction hash command (OFFLINE)
///
/// Hash a transaction to get it's identifier for lookup in mempool
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionhash)
pub(crate) async fn construction_hash(
    request: ConstructionHashRequest,
    server_context: RosettaContext,
) -> ApiResult<TransactionIdentifierResponse> {
    debug!("/construction/hash {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Decode the SignedTransaction and hash it accordingly.  This in theory works for any transaction
    // but it is expected to only be UserTransactions
    let signed_transaction: SignedTransaction =
        decode_bcs(&request.signed_transaction, "SignedTransaction")?;

    Ok(TransactionIdentifierResponse {
        transaction_identifier: signed_transaction.committed_hash().into(),
    })
}
