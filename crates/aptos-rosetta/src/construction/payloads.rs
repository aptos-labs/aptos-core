// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{check_network, encode_bcs},
    error::{ApiError, ApiResult},
    types::*,
    RosettaContext,
};
use aptos_crypto::signing_message;
use aptos_logger::debug;
use aptos_sdk::transaction_builder::TransactionFactory;

/// Construction payloads command (OFFLINE)
///
/// Constructs payloads for given known operations.  This converts Rosetta [Operation]s to a [RawTransaction]
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionpayloads)
pub(crate) async fn construction_payloads(
    request: ConstructionPayloadsRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionPayloadsResponse> {
    debug!("/construction/payloads {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Retrieve the real operation we're doing, this identifies the sub-operations to a function
    let mut operation = InternalOperation::extract(&server_context, &request.operations)?;

    // For some reason, metadata is optional on the Rosetta spec, we enforce it here, otherwise we
    // can't build the [RawTransaction] offline.
    let metadata = if let Some(ref metadata) = request.metadata {
        metadata
    } else {
        return Err(ApiError::MissingPayloadMetadata);
    };

    // This is a hack to ensure that the payloads actually have overridden operators if not provided
    // It ensures that the operations provided match the metadata provided.
    // TODO: Move this to a separate function
    match &mut operation {
        InternalOperation::CreateAccount(_) => {
            if operation != metadata.internal_operation {
                return Err(ApiError::InvalidInput(Some(format!(
                    "CreateAccount operation doesn't match metadata {:?} vs {:?}",
                    operation, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::Transfer(_) => {
            if operation != metadata.internal_operation {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Transfer operation doesn't match metadata {:?} vs {:?}",
                    operation, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::SetOperator(inner) => {
            if let InternalOperation::SetOperator(ref metadata_op) = metadata.internal_operation {
                if inner.owner == metadata_op.owner
                    && inner.new_operator == metadata_op.new_operator
                {
                    if inner.old_operator.is_none() {
                        inner.old_operator = metadata_op.old_operator;
                    }
                } else {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Set operator operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Set operator operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::SetVoter(inner) => {
            if let InternalOperation::SetVoter(ref metadata_op) = metadata.internal_operation {
                if inner.owner == metadata_op.owner && inner.new_voter == metadata_op.new_voter {
                    if inner.operator.is_none() {
                        inner.operator = metadata_op.operator;
                    }
                } else {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Set voter operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Set voter operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::InitializeStakePool(_) => {
            if operation != metadata.internal_operation {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Initialize stake pool doesn't match metadata {:?} vs {:?}",
                    operation, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::ResetLockup(inner) => {
            if let InternalOperation::ResetLockup(ref metadata_op) = metadata.internal_operation {
                if inner.owner != metadata_op.owner || inner.operator != metadata_op.operator {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Reset lockup operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Reset lockup operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::UnlockStake(inner) => {
            if let InternalOperation::UnlockStake(ref metadata_op) = metadata.internal_operation {
                if inner.owner != metadata_op.owner || inner.operator != metadata_op.operator {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Unlock stake operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Unlock stake operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::UpdateCommission(inner) => {
            if let InternalOperation::UpdateCommission(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.owner != metadata_op.owner || inner.operator != metadata_op.operator {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Update commission operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Update commission operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::DistributeStakingRewards(inner) => {
            if let InternalOperation::DistributeStakingRewards(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.operator != metadata_op.operator || inner.staker != metadata_op.staker {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Distribute staking rewards operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Distribute staking rewards operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::AddDelegatedStake(inner) => {
            if let InternalOperation::AddDelegatedStake(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.delegator != metadata_op.delegator
                    || inner.pool_address != metadata_op.pool_address
                {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "AddDelegatedStake internal operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "InternalOperation::AddDelegatedStake doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::UnlockDelegatedStake(inner) => {
            if let InternalOperation::UnlockDelegatedStake(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.delegator != metadata_op.delegator
                    || inner.pool_address != metadata_op.pool_address
                {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Unlock delegated stake operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Unlock delegated stake operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::WithdrawUndelegated(inner) => {
            if let InternalOperation::WithdrawUndelegated(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.delegator != metadata_op.delegator
                    || inner.pool_address != metadata_op.pool_address
                {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Withdraw undelegated operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Withdraw undelegated operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
    }

    // Encode operation
    let (txn_payload, sender) = operation.payload()?;

    // Build the transaction and make it ready for signing
    let transaction_factory = TransactionFactory::new(server_context.chain_id)
        .with_gas_unit_price(metadata.gas_price_per_unit.0)
        .with_max_gas_amount(metadata.max_gas_amount.0);

    let mut txn_builder = transaction_factory
        .payload(txn_payload)
        .sender(sender)
        .sequence_number(metadata.sequence_number.0);

    // Default expiry is 30 seconds from right now
    if let Some(expiry_time_secs) = metadata.expiry_time_secs {
        txn_builder = txn_builder.expiration_timestamp_secs(expiry_time_secs.0)
    }
    let unsigned_transaction = txn_builder.build();

    // Build a signing message so that an external signer can sign with Ed25519 without knowing BCS
    let signing_message = hex::encode(signing_message(&unsigned_transaction).map_err(|err| {
        ApiError::InvalidInput(Some(format!(
            "Invalid transaction, can't build into a signing message {}",
            err
        )))
    })?);
    let payload = SigningPayload {
        account_identifier: AccountIdentifier::base_account(sender),
        hex_bytes: signing_message,
        signature_type: Some(SignatureType::Ed25519),
    };

    // Transaction is both the unsigned transaction and the payload
    Ok(ConstructionPayloadsResponse {
        unsigned_transaction: encode_bcs(&unsigned_transaction)?,
        payloads: vec![payload],
    })
}
