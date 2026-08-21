// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use crate::{
    common::{find_coin_currency, find_fa_currency, native_coin},
    construction::{
        parse_create_stake_pool_operation, parse_delegation_pool_add_stake_operation,
        parse_delegation_pool_unlock_operation, parse_delegation_pool_withdraw_operation,
        parse_distribute_staking_rewards_operation, parse_reset_lockup_operation,
        parse_set_operator_operation, parse_set_voter_operation, parse_unlock_stake_operation,
        parse_update_commission_operation,
    },
    error::ApiResult,
    types::{move_types::*, AccountIdentifier, OperationStatusType, TransactionIdentifier},
    RosettaContext,
};
use aptos_logger::warn;
use aptos_rest_client::aptos_api_types::{TransactionOnChainData, U64};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
};

/// A representation of a transaction by it's underlying operations (write set changes)
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Transaction.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transaction {
    /// The identifying hash of the transaction
    pub transaction_identifier: TransactionIdentifier,
    /// Individual operations (write set changes) in a transaction
    pub operations: Vec<Operation>,
    pub metadata: TransactionMetadata,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionMetadata {
    pub transaction_type: TransactionType,
    pub version: U64,
    pub failed: bool,
    pub vm_status: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionType {
    User,
    Genesis,
    BlockMetadata,
    BlockMetadataExt,
    StateCheckpoint,
    Validator,
    BlockEpilogue,
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use TransactionType::*;
        f.write_str(match self {
            User => "User",
            Genesis => "Genesis",
            BlockMetadata => "BlockResource",
            BlockMetadataExt => "BlockResourceExt",
            StateCheckpoint => "StateCheckpoint",
            Validator => "Validator",
            BlockEpilogue => "BlockEpilogue",
        })
    }
}

impl Transaction {
    pub async fn from_transaction(
        server_context: &RosettaContext,
        txn: TransactionOnChainData,
    ) -> ApiResult<Transaction> {
        // Parses the events, changesets, and metadata out of each transaction
        use aptos_types::transaction::Transaction::*;
        let (txn_type, maybe_user_txn, txn_info, events) = match &txn.transaction {
            UserTransaction(user_txn) => {
                (TransactionType::User, Some(user_txn), txn.info, txn.events)
            },
            GenesisTransaction(_) => (TransactionType::Genesis, None, txn.info, txn.events),
            BlockMetadata(_) => (TransactionType::BlockMetadata, None, txn.info, txn.events),
            BlockMetadataExt(_) => (
                TransactionType::BlockMetadataExt,
                None,
                txn.info,
                txn.events,
            ),
            StateCheckpoint(_) => (TransactionType::StateCheckpoint, None, txn.info, vec![]),
            ValidatorTransaction(_) => (TransactionType::Validator, None, txn.info, txn.events),
            BlockEpilogue(_) => (TransactionType::BlockEpilogue, None, txn.info, vec![]),
        };

        let gas_payer = maybe_user_txn.map(|txn| {
            txn.authenticator()
                .fee_payer_address()
                .unwrap_or_else(|| txn.sender())
        });

        // Operations must be sequential and operation index must always be in the same order
        // with no gaps
        let successful = txn_info.status().is_success();
        let mut operations = vec![];
        let mut operation_index: u64 = 0;
        if successful {
            let mut object_to_owner = HashMap::new();
            let mut store_to_currency = HashMap::new();
            let mut framework_changes = vec![];
            // Not the most efficient, parse all store owners, and assets associated with stores
            for (state_key, write_op) in txn.changes.write_op_iter() {
                let new_changes = preprocess_write_set(
                    server_context,
                    state_key,
                    write_op,
                    maybe_user_txn.map(|inner| inner.payload()),
                    txn.version,
                    &mut object_to_owner,
                    &mut store_to_currency,
                );
                framework_changes.extend(new_changes);
            }

            // Parse all operations from the writeset changes in a success
            for (struct_tag, account_address, data) in &framework_changes {
                let mut ops = parse_operations_from_write_set(
                    server_context,
                    struct_tag,
                    *account_address,
                    data,
                    &events, // TODO: Filter events down to framework events only
                    maybe_user_txn.map(|inner| inner.sender()),
                    txn.version,
                    operation_index,
                    &txn.changes, // TODO: Move to parsed framework_changes
                    &mut object_to_owner,
                    &mut store_to_currency,
                )
                .await?;
                operation_index += ops.len() as u64;
                operations.append(&mut ops);
            }

            // For storage fee refund
            if let Some(refund_recipient) = gas_payer {
                let fee_events = get_fee_statement_from_event(&events)
                    .into_iter()
                    .filter(|event| event.storage_fee_refund() > 0);
                for event in fee_events {
                    operations.push(Operation::deposit(
                        operation_index,
                        Some(OperationStatusType::Success),
                        AccountIdentifier::base_account(refund_recipient),
                        native_coin(),
                        event.storage_fee_refund(),
                    ));
                    operation_index += 1;
                }
            }
        } else {
            // Parse all failed operations from the payload
            if let Some(user_txn) = maybe_user_txn {
                let mut ops = parse_failed_operations_from_txn_payload(
                    &server_context.currencies,
                    operation_index,
                    user_txn.sender(),
                    user_txn.payload(),
                );
                operation_index += ops.len() as u64;
                operations.append(&mut ops);
            }
        };

        // Reorder operations by type so that there's no invalid ordering
        // (Create before transfer) (Withdraw before deposit)
        operations.sort();
        for (i, operation) in operations.iter_mut().enumerate() {
            operation.operation_identifier.index = i as u64;
        }

        // Everything committed costs gas
        if let Some((payer, txn)) = gas_payer.zip(maybe_user_txn) {
            operations.push(Operation::gas_fee(
                operation_index,
                payer,
                txn_info.gas_used(),
                txn.gas_unit_price(),
            ));
        }

        // TODO: Handle storage gas refund (though nothing currently in Rosetta refunds)

        Ok(Transaction {
            transaction_identifier: (&txn_info).into(),
            operations,
            metadata: TransactionMetadata {
                transaction_type: txn_type,
                version: txn.version.into(),
                failed: !successful,
                vm_status: format!("{:?}", txn_info.status()),
            },
        })
    }
}

/// Parses operations from the transaction payload
///
/// This case only occurs if the transaction failed, and that's because it's less accurate
/// than just following the state changes
fn parse_failed_operations_from_txn_payload(
    currencies: &HashSet<Currency>,
    operation_index: u64,
    sender: AccountAddress,
    payload: &TransactionPayload,
) -> Vec<Operation> {
    let mut operations = vec![];
    if let TransactionPayload::EntryFunction(inner) = payload {
        match (
            *inner.module().address(),
            inner.module().name().as_str(),
            inner.function().as_str(),
        ) {
            (AccountAddress::ONE, COIN_MODULE, TRANSFER_FUNCTION)
            | (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_COINS_FUNCTION) => {
                // We could add a create here as well on transfer_coins, but we don't know if it will actually happen
                if let Some(type_tag) = inner.ty_args().first() {
                    // Find currency from type tag
                    let maybe_currency = find_coin_currency(currencies, type_tag);

                    if let Some(currency) = maybe_currency {
                        operations = parse_coin_transfer_from_txn_payload(
                            inner,
                            currency.clone(),
                            sender,
                            operation_index,
                        )
                    }
                }
            },
            (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNCTION) => {
                // We could add a create here as well, but we don't know if it will actually happen
                operations = parse_coin_transfer_from_txn_payload(
                    inner,
                    native_coin(),
                    sender,
                    operation_index,
                )
            },
            (AccountAddress::ONE, PRIMARY_FUNGIBLE_STORE_MODULE, TRANSFER_FUNCTION)
            | (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNGIBLE_ASSETS_FUNCTION) => {
                // Primary transfer has the same interface as coin transfer, but it's a metadata address instead of a coin type generic
                let maybe_metadata_address = inner
                    .args()
                    .first()
                    .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded));
                if let Some(Ok(addr)) = maybe_metadata_address {
                    // Find currency from type tag
                    let maybe_currency = find_fa_currency(currencies, addr);

                    if let Some(currency) = maybe_currency {
                        operations = parse_primary_fa_transfer_from_txn_payload(
                            inner,
                            currency.clone(),
                            sender,
                            operation_index,
                        )
                    }
                }
            },
            (AccountAddress::ONE, DISPATCHABLE_FUNGIBLE_ASSET_MODULE, TRANSFER_FUNCTION) => {
                // TODO: This isn't really easy to handle atm, objects get messy, need owners etc.
            },
            (AccountAddress::ONE, ACCOUNT_MODULE, CREATE_ACCOUNT_FUNCTION) => {
                if let Some(Ok(address)) = inner
                    .args()
                    .first()
                    .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded))
                {
                    operations.push(Operation::create_account(
                        operation_index,
                        Some(OperationStatusType::Failure),
                        address,
                        sender,
                    ));
                } else {
                    warn!("Failed to parse create account {:?}", inner);
                }
            },
            (
                AccountAddress::ONE,
                STAKING_CONTRACT_MODULE,
                SWITCH_OPERATOR_WITH_SAME_COMMISSION_FUNCTION,
            ) => {
                if let Ok(mut ops) =
                    parse_set_operator_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse set operator {:?}", inner);
                }
            },
            (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UPDATE_VOTER_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_set_voter_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse set voter {:?}", inner);
                }
            },
            (AccountAddress::ONE, STAKING_CONTRACT_MODULE, RESET_LOCKUP_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_reset_lockup_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse reset lockup {:?}", inner);
                }
            },
            (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UPDATE_COMMISSION_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_update_commission_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse update commission {:?}", inner);
                }
            },
            (AccountAddress::ONE, STAKING_CONTRACT_MODULE, CREATE_STAKING_CONTRACT_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_create_stake_pool_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse create staking pool {:?}", inner);
                }
            },
            (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UNLOCK_STAKE_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_unlock_stake_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse unlock stake {:?}", inner);
                }
            },
            (AccountAddress::ONE, STAKING_CONTRACT_MODULE, DISTRIBUTE_STAKING_REWARDS_FUNCTION) => {
                if let Ok(mut ops) = parse_distribute_staking_rewards_operation(
                    sender,
                    inner.ty_args(),
                    inner.args(),
                ) {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse distribute staking rewards {:?}", inner);
                }
            },
            (AccountAddress::ONE, DELEGATION_POOL_MODULE, DELEGATION_POOL_ADD_STAKE_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_delegation_pool_add_stake_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse delegation_pool::add_stake {:?}", inner);
                }
            },
            (AccountAddress::ONE, DELEGATION_POOL_MODULE, DELEGATION_POOL_WITHDRAW_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_delegation_pool_withdraw_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse delegation_pool::withdraw {:?}", inner);
                }
            },
            (AccountAddress::ONE, DELEGATION_POOL_MODULE, DELEGATION_POOL_UNLOCK_FUNCTION) => {
                if let Ok(mut ops) =
                    parse_delegation_pool_unlock_operation(sender, inner.ty_args(), inner.args())
                {
                    if let Some(operation) = ops.get_mut(0) {
                        operation.status = Some(OperationStatusType::Failure.to_string());
                    }
                } else {
                    warn!("Failed to parse delegation_pool::unlock {:?}", inner);
                }
            },
            _ => {
                // If we don't recognize the transaction payload, then we can't parse operations
            },
        }
    }
    operations
}

/// Parses a 0x1::coin::transfer to a Withdraw and Deposit
fn parse_coin_transfer_from_txn_payload(
    payload: &EntryFunction,
    currency: Currency,
    sender: AccountAddress,
    operation_index: u64,
) -> Vec<Operation> {
    let args = payload.args();
    let maybe_receiver = args
        .first()
        .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded));
    let maybe_amount = args.get(1).map(|encoded| bcs::from_bytes::<u64>(encoded));

    build_transfer_operations(
        payload,
        operation_index,
        sender,
        maybe_receiver,
        maybe_amount,
        currency,
    )
}

/// Parses a 0x1::primary_fungible_store::transfer to a Withdraw and Deposit
fn parse_primary_fa_transfer_from_txn_payload(
    payload: &EntryFunction,
    currency: Currency,
    sender: AccountAddress,
    operation_index: u64,
) -> Vec<Operation> {
    let args = payload.args();
    let maybe_receiver = args
        .get(1)
        .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded));
    let maybe_amount = args.get(2).map(|encoded| bcs::from_bytes::<u64>(encoded));

    build_transfer_operations(
        payload,
        operation_index,
        sender,
        maybe_receiver,
        maybe_amount,
        currency,
    )
}

/// Builds operations for a coin or FA transfer
fn build_transfer_operations(
    payload: &EntryFunction,
    operation_index: u64,
    sender: AccountAddress,
    maybe_receiver: Option<Result<AccountAddress, bcs::Error>>,
    maybe_amount: Option<Result<u64, bcs::Error>>,
    currency: Currency,
) -> Vec<Operation> {
    let mut operations = vec![];

    if let (Some(Ok(receiver)), Some(Ok(amount))) = (maybe_receiver, maybe_amount) {
        operations.push(Operation::withdraw(
            operation_index,
            Some(OperationStatusType::Failure),
            AccountIdentifier::base_account(sender),
            currency.clone(),
            amount,
        ));
        operations.push(Operation::deposit(
            operation_index + 1,
            Some(OperationStatusType::Failure),
            AccountIdentifier::base_account(receiver),
            currency,
            amount,
        ));
    } else {
        warn!(
            "Failed to parse account's {} transfer {:?}",
            sender, payload
        );
    }

    operations
}
