// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::check_network,
    error::{ApiError, ApiResult},
    types::*,
    RosettaContext,
};
use aptos_logger::debug;
use std::time::{SystemTime, UNIX_EPOCH};

/// Construction preprocess command (OFFLINE)
///
/// This creates the request needed to fetch metadata.  It basically verifies that the inputs are
/// valid for calling on-chain data.
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionpreprocess)
pub(crate) async fn construction_preprocess(
    request: ConstructionPreprocessRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionPreprocessResponse> {
    debug!("/construction/preprocess {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Determine the actual operation from the collection of Rosetta [Operation]
    let internal_operation = InternalOperation::extract(&server_context, &request.operations)?;

    // Provide the accounts that need public keys (there's only one supported today)
    let required_public_keys = vec![AccountIdentifier::base_account(internal_operation.sender())];

    // Verify that the max gas value is valid
    if let Some(max_gas) = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.max_gas_amount)
    {
        if max_gas.0 < 1 {
            return Err(ApiError::InvalidInput(Some(
                "Cannot have a max gas amount less than 1".to_string(),
            )));
        }
    }

    // Verify that expiration time is valid
    if let Some(expiry_time_secs) = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.expiry_time_secs)
    {
        // Probably should be greater than now + some amount of time, but for now it's valid
        if expiry_time_secs.0
            <= SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|err| {
                    ApiError::InternalError(Some(format!("Failed to get current time {}", err)))
                })?
                .as_secs()
        {
            return Err(ApiError::InvalidInput(Some(
                "Expiry time secs is in the past, please provide a Unix timestamp in the future"
                    .to_string(),
            )));
        }
    }

    // Check gas input options
    let public_keys = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.public_keys.as_ref());

    // A public key can be provided for simulation, otherwise, a max gas amount would be given.
    if request
        .metadata
        .as_ref()
        .and_then(|inner| inner.max_gas_amount)
        .is_none()
        && public_keys
            .as_ref()
            .map(|inner| inner.is_empty())
            .unwrap_or(false)
    {
        return Err(ApiError::InvalidInput(Some(
            "Must provide either max gas amount or public keys to estimate max gas amount"
                .to_string(),
        )));
    }

    // Convert it to an input to the metadata call
    // TODO: Refactor so that it only does `request.metadata.as_ref()` once
    Ok(ConstructionPreprocessResponse {
        options: MetadataOptions {
            internal_operation,
            max_gas_amount: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.max_gas_amount),
            gas_price_per_unit: request.metadata.as_ref().and_then(|inner| inner.gas_price),
            expiry_time_secs: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.expiry_time_secs),
            sequence_number: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.sequence_number),
            public_keys: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.public_keys.clone()),
            gas_price_multiplier: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.gas_price_multiplier),
            gas_price_priority: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.gas_price_priority),
        },
        required_public_keys,
    })
}
