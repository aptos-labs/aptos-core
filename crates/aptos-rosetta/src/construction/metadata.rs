// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use crate::{
    common::{check_network, get_account},
    error::{ApiError, ApiResult},
    types::*,
    RosettaContext,
};
use aptos_logger::debug;

/// Construction metadata command
///
/// Retrieves sequence number, gas price, max gas, gas estimate for the transaction
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionmetadata)
pub(crate) async fn construction_metadata(
    request: ConstructionMetadataRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionMetadataResponse> {
    debug!("/construction/metadata {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    let rest_client = server_context.rest_client()?;
    let address = request.options.internal_operation.sender();
    let response = get_account(rest_client.as_ref(), address).await?;

    // Ensure this network really is the one we expect it to be
    if server_context.chain_id.id() != response.state().chain_id {
        return Err(ApiError::ChainIdMismatch);
    }

    // Retrieve the sequence number from the rest server if one wasn't provided
    let sequence_number = if let Some(sequence_number) = request.options.sequence_number {
        sequence_number.0
    } else {
        response.inner().sequence_number
    };

    // We have to cheat the set operator and set voter operations right here
    let internal_operation = fill_in_operator(
        rest_client.as_ref(),
        request.options.internal_operation.clone(),
    )
    .await?;

    // If both are present, we skip simulation
    let (suggested_fee, gas_unit_price, max_gas_amount) = simulate_transaction(
        rest_client.as_ref(),
        server_context.chain_id,
        &request.options,
        &internal_operation,
        sequence_number,
    )
    .await?;

    Ok(ConstructionMetadataResponse {
        metadata: ConstructionMetadata {
            sequence_number: sequence_number.into(),
            max_gas_amount: max_gas_amount.into(),
            gas_price_per_unit: gas_unit_price.into(),
            expiry_time_secs: request.options.expiry_time_secs,
            internal_operation,
        },
        suggested_fee: vec![suggested_fee],
    })
}
