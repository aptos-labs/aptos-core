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

/// Construction submit command (OFFLINE)
///
/// Submits a transaction to the blockchain
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionsubmit)
pub(crate) async fn construction_submit(
    request: ConstructionSubmitRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionSubmitResponse> {
    debug!("/construction/submit {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    let rest_client = server_context.rest_client()?;

    // Submits the transaction, and returns the hash of the transaction
    let txn: SignedTransaction = decode_bcs(&request.signed_transaction, "SignedTransaction")?;
    let hash = txn.committed_hash();
    rest_client.submit_bcs(&txn).await?;
    Ok(ConstructionSubmitResponse {
        transaction_identifier: hash.into(),
    })
}
