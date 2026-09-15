// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use crate::{
    common::{find_coin_currency, find_fa_currency, native_coin},
    error::ApiResult,
    types::{
        move_types::*, AccountIdentifier, OperationIdentifier, OperationStatusType, OperationType,
    },
    RosettaContext,
};
use aptos_logger::warn;
use aptos_rest_client::aptos_api_types::{ResourceGroup, U64};
use aptos_types::{
    access_path::Path,
    account_address::{create_derived_object_address, AccountAddress},
    account_config::{
        fungible_store::FungibleStoreResource, AccountResource, CoinStoreResourceUntyped,
        CoinWithdraw, DepositFAEvent, ObjectCoreResource, WithdrawEvent,
    },
    contract_event::{ContractEvent, ContractEventV2, FEE_STATEMENT_EVENT_TYPE},
    event::EventKey,
    fee_statement::FeeStatement,
    stake_pool::{SetOperatorEvent, StakePool},
    state_store::state_key::{inner::StateKeyInner, StateKey},
    transaction::TransactionPayload,
    write_set::{WriteOp, WriteSet},
};
use itertools::Itertools;
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    parser::parse_type_tag,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    str::FromStr,
};

static WITHDRAW_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::fungible_asset::Withdraw").unwrap());
static DEPOSIT_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::fungible_asset::Deposit").unwrap());

static COIN_WITHDRAW_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::coin::CoinWithdraw").unwrap());
static COIN_DEPOSIT_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::coin::CoinDeposit").unwrap());

static SET_OPERATOR_EVENT_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::stake::SetOperator").unwrap());
static UPDATE_VOTER_EVENT_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::staking_contract::UpdateVoter").unwrap());
static DISTRIBUTE_STAKING_REWARDS_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::staking_contract::Distribute").unwrap());
static UPDATE_COMMISSION_TAG: Lazy<TypeTag> =
    Lazy::new(|| parse_type_tag("0x1::staking_contract::UpdateCommission").unwrap());

/// A representation of a single account change in a transaction
///
/// This is known as a write set change within Aptos
/// [API Spec](https://www.rosetta-api.org/docs/models/Operation.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Operation {
    /// Identifier of an operation within a transaction
    pub operation_identifier: OperationIdentifier,
    /// Type of operation
    #[serde(rename = "type")]
    pub operation_type: String,
    /// Status of operation.  Must be populated if the transaction is in the past.  If submitting
    /// new transactions, it must NOT be populated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// AccountIdentifier should be provided to point at which account the change is
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<AccountIdentifier>,
    /// Amount in the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    /// Operation specific metadata for any operation that's missing information it needs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<OperationMetadata>,
}

impl Operation {
    fn new(
        operation_type: OperationType,
        operation_index: u64,
        status: Option<OperationStatusType>,
        account: AccountIdentifier,
        amount: Option<Amount>,
        metadata: Option<OperationMetadata>,
    ) -> Operation {
        Operation {
            operation_identifier: OperationIdentifier {
                index: operation_index,
            },
            operation_type: operation_type.to_string(),
            status: status.map(|inner| inner.to_string()),
            account: Some(account),
            amount,
            metadata,
        }
    }

    pub fn create_stake_pool(
        operation_index: u64,
        status: Option<OperationStatusType>,
        owner: AccountAddress,
        operator: Option<AccountAddress>,
        voter: Option<AccountAddress>,
        staked_balance: Option<u64>,
        commission_percentage: Option<u64>,
    ) -> Operation {
        Operation::new(
            OperationType::InitializeStakePool,
            operation_index,
            status,
            AccountIdentifier::base_account(owner),
            None,
            Some(OperationMetadata::create_stake_pool(
                operator.map(AccountIdentifier::base_account),
                voter.map(AccountIdentifier::base_account),
                staked_balance,
                commission_percentage,
            )),
        )
    }

    pub fn create_account(
        operation_index: u64,
        status: Option<OperationStatusType>,
        address: AccountAddress,
        sender: AccountAddress,
    ) -> Operation {
        Operation::new(
            OperationType::CreateAccount,
            operation_index,
            status,
            AccountIdentifier::base_account(address),
            None,
            Some(OperationMetadata::create_account(sender)),
        )
    }

    pub fn staking_reward(
        operation_index: u64,
        status: Option<OperationStatusType>,
        account: AccountIdentifier,
        currency: Currency,
        amount: u64,
    ) -> Operation {
        Operation::new(
            OperationType::StakingReward,
            operation_index,
            status,
            account,
            Some(Amount {
                value: amount.to_string(),
                currency,
            }),
            None,
        )
    }

    pub fn deposit(
        operation_index: u64,
        status: Option<OperationStatusType>,
        account: AccountIdentifier,
        currency: Currency,
        amount: u64,
    ) -> Operation {
        Operation::new(
            OperationType::Deposit,
            operation_index,
            status,
            account,
            Some(Amount {
                value: amount.to_string(),
                currency,
            }),
            None,
        )
    }

    pub fn withdraw(
        operation_index: u64,
        status: Option<OperationStatusType>,
        account: AccountIdentifier,
        currency: Currency,
        amount: u64,
    ) -> Operation {
        Operation::new(
            OperationType::Withdraw,
            operation_index,
            status,
            account,
            Some(Amount {
                value: format!("-{}", amount),
                currency,
            }),
            None,
        )
    }

    pub fn gas_fee(
        operation_index: u64,
        address: AccountAddress,
        gas_used: u64,
        gas_price_per_unit: u64,
    ) -> Operation {
        Operation::new(
            OperationType::Fee,
            operation_index,
            Some(OperationStatusType::Success),
            AccountIdentifier::base_account(address),
            Some(Amount {
                value: format!("-{}", gas_used.saturating_mul(gas_price_per_unit)),
                currency: native_coin(),
            }),
            None,
        )
    }

    pub fn set_operator(
        operation_index: u64,
        status: Option<OperationStatusType>,
        owner: AccountAddress,
        old_operator: Option<AccountIdentifier>,
        new_operator: AccountIdentifier,
        staked_balance: Option<u64>,
    ) -> Operation {
        Operation::new(
            OperationType::SetOperator,
            operation_index,
            status,
            AccountIdentifier::base_account(owner),
            None,
            Some(OperationMetadata::set_operator(
                old_operator,
                new_operator,
                staked_balance,
            )),
        )
    }

    pub fn set_voter(
        operation_index: u64,
        status: Option<OperationStatusType>,
        owner: AccountAddress,
        operator: Option<AccountIdentifier>,
        new_voter: AccountIdentifier,
    ) -> Operation {
        Operation::new(
            OperationType::SetVoter,
            operation_index,
            status,
            AccountIdentifier::base_account(owner),
            None,
            Some(OperationMetadata::set_voter(operator, new_voter)),
        )
    }

    pub fn reset_lockup(
        operation_index: u64,
        status: Option<OperationStatusType>,
        owner: AccountAddress,
        operator: Option<AccountIdentifier>,
    ) -> Operation {
        Operation::new(
            OperationType::ResetLockup,
            operation_index,
            status,
            AccountIdentifier::base_account(owner),
            None,
            Some(OperationMetadata::reset_lockup(operator)),
        )
    }

    pub fn unlock_stake(
        operation_index: u64,
        status: Option<OperationStatusType>,
        owner: AccountAddress,
        operator: Option<AccountIdentifier>,
        amount: Option<u64>,
    ) -> Operation {
        Operation::new(
            OperationType::UnlockStake,
            operation_index,
            status,
            AccountIdentifier::base_account(owner),
            None,
            Some(OperationMetadata::unlock_stake(operator, amount)),
        )
    }

    pub fn update_commission(
        operation_index: u64,
        status: Option<OperationStatusType>,
        owner: AccountAddress,
        operator: Option<AccountIdentifier>,
        new_commission_percentage: Option<u64>,
    ) -> Operation {
        Operation::new(
            OperationType::UpdateCommission,
            operation_index,
            status,
            AccountIdentifier::base_account(owner),
            None,
            Some(OperationMetadata::update_commission(
                operator,
                new_commission_percentage,
            )),
        )
    }

    pub fn distribute_staking_rewards(
        operation_index: u64,
        status: Option<OperationStatusType>,
        account: AccountAddress,
        operator: AccountIdentifier,
        staker: AccountIdentifier,
    ) -> Operation {
        Operation::new(
            OperationType::DistributeStakingRewards,
            operation_index,
            status,
            AccountIdentifier::base_account(account),
            None,
            Some(OperationMetadata::distribute_staking_rewards(
                operator, staker,
            )),
        )
    }

    pub fn account(&self) -> Option<AccountAddress> {
        self.account
            .as_ref()
            .and_then(|inner| inner.account_address().ok())
    }

    pub fn currency(&self) -> Option<&Currency> {
        self.amount.as_ref().map(|inner| &inner.currency)
    }

    pub fn amount(&self) -> Option<i128> {
        self.amount.as_ref().and_then(|inner| inner.value().ok())
    }

    pub fn status(&self) -> Option<OperationStatusType> {
        self.status
            .as_ref()
            .and_then(|inner| OperationStatusType::from_str(inner).ok())
    }

    pub fn operation_type(&self) -> Option<OperationType> {
        OperationType::from_str(&self.operation_type).ok()
    }

    pub fn operator(&self) -> Option<AccountAddress> {
        self.metadata.as_ref().and_then(|inner| {
            inner
                .operator
                .as_ref()
                .and_then(|inner| inner.account_address().ok())
        })
    }

    pub fn old_operator(&self) -> Option<AccountAddress> {
        self.metadata.as_ref().and_then(|inner| {
            inner
                .old_operator
                .as_ref()
                .and_then(|inner| inner.account_address().ok())
        })
    }

    pub fn new_operator(&self) -> Option<AccountAddress> {
        self.metadata.as_ref().and_then(|inner| {
            inner
                .new_operator
                .as_ref()
                .and_then(|inner| inner.account_address().ok())
        })
    }

    pub fn sender(&self) -> Option<AccountAddress> {
        self.metadata.as_ref().and_then(|inner| {
            inner
                .sender
                .as_ref()
                .and_then(|inner| inner.account_address().ok())
        })
    }

    pub fn staker(&self) -> Option<AccountAddress> {
        self.metadata.as_ref().and_then(|inner| {
            inner
                .staker
                .as_ref()
                .and_then(|inner| inner.account_address().ok())
        })
    }

    pub fn new_voter(&self) -> Option<AccountAddress> {
        self.metadata.as_ref().and_then(|inner| {
            inner
                .new_voter
                .as_ref()
                .and_then(|inner| inner.account_address().ok())
        })
    }

    pub fn metadata_amount(&self) -> Option<u64> {
        self.metadata
            .as_ref()
            .and_then(|inner| inner.amount.map(|inner| inner.0))
    }

    pub fn staked_balance(&self) -> Option<u64> {
        self.metadata
            .as_ref()
            .and_then(|inner| inner.staked_balance.map(|inner| inner.0))
    }

    pub fn commission_percentage(&self) -> Option<u64> {
        self.metadata
            .as_ref()
            .and_then(|inner| inner.commission_percentage.map(|inner| inner.0))
    }

    pub fn add_delegated_stake(
        operation_index: u64,
        status: Option<OperationStatusType>,
        delegator: AccountAddress,
        pool_address: AccountIdentifier,
        amount: Option<u64>,
    ) -> Operation {
        Operation::new(
            OperationType::AddDelegatedStake,
            operation_index,
            status,
            AccountIdentifier::base_account(delegator),
            None,
            Some(OperationMetadata::add_delegated_stake(pool_address, amount)),
        )
    }

    pub fn unlock_delegated_stake(
        operation_index: u64,
        status: Option<OperationStatusType>,
        delegator: AccountAddress,
        pool_address: AccountIdentifier,
        amount: Option<u64>,
    ) -> Operation {
        Operation::new(
            OperationType::UnlockDelegatedStake,
            operation_index,
            status,
            AccountIdentifier::base_account(delegator),
            None,
            Some(OperationMetadata::unlock_delegated_stake(
                pool_address,
                amount,
            )),
        )
    }

    pub fn withdraw_undelegated_stake(
        operation_index: u64,
        status: Option<OperationStatusType>,
        owner: AccountAddress,
        pool_address: AccountIdentifier,
        amount: Option<u64>,
    ) -> Operation {
        Operation::new(
            OperationType::WithdrawUndelegatedFunds,
            operation_index,
            status,
            AccountIdentifier::base_account(owner),
            None,
            Some(OperationMetadata::withdraw_undelegated_stake(
                pool_address,
                amount,
            )),
        )
    }
}

impl std::cmp::PartialOrd for Operation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Operation {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_op = OperationType::from_str(&self.operation_type).ok();
        let other_op = OperationType::from_str(&other.operation_type).ok();
        match (self_op, other_op) {
            (Some(self_op), Some(other_op)) => {
                match self_op.cmp(&other_op) {
                    // Keep the order stable if there's a difference
                    Ordering::Equal => self
                        .operation_identifier
                        .index
                        .cmp(&other.operation_identifier.index),
                    order => order,
                }
            },
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}

/// This object is needed for flattening all the types into a
/// single json object used by Rosetta
#[derive(Clone, Default, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<AccountIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<AccountIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_operator: Option<AccountIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_operator: Option<AccountIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_voter: Option<AccountIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staked_balance: Option<U64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commission_percentage: Option<U64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<U64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staker: Option<AccountIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_address: Option<AccountIdentifier>,
}

impl OperationMetadata {
    pub fn create_account(sender: AccountAddress) -> Self {
        OperationMetadata {
            sender: Some(AccountIdentifier::base_account(sender)),
            ..Default::default()
        }
    }

    pub fn set_operator(
        old_operator: Option<AccountIdentifier>,
        new_operator: AccountIdentifier,
        staked_balance: Option<u64>,
    ) -> Self {
        OperationMetadata {
            old_operator,
            new_operator: Some(new_operator),
            staked_balance: staked_balance.map(U64::from),
            ..Default::default()
        }
    }

    pub fn set_voter(operator: Option<AccountIdentifier>, new_voter: AccountIdentifier) -> Self {
        OperationMetadata {
            operator,
            new_voter: Some(new_voter),
            ..Default::default()
        }
    }

    pub fn create_stake_pool(
        new_operator: Option<AccountIdentifier>,
        new_voter: Option<AccountIdentifier>,
        staked_balance: Option<u64>,
        commission_percentage: Option<u64>,
    ) -> Self {
        OperationMetadata {
            new_operator,
            new_voter,
            staked_balance: staked_balance.map(U64::from),
            commission_percentage: commission_percentage.map(U64::from),
            ..Default::default()
        }
    }

    pub fn reset_lockup(operator: Option<AccountIdentifier>) -> Self {
        OperationMetadata {
            operator,
            ..Default::default()
        }
    }

    pub fn unlock_stake(operator: Option<AccountIdentifier>, amount: Option<u64>) -> Self {
        OperationMetadata {
            operator,
            amount: amount.map(U64::from),
            ..Default::default()
        }
    }

    pub fn update_commission(
        operator: Option<AccountIdentifier>,
        new_commission_percentage: Option<u64>,
    ) -> Self {
        OperationMetadata {
            operator,
            commission_percentage: new_commission_percentage.map(U64::from),
            ..Default::default()
        }
    }

    pub fn distribute_staking_rewards(
        operator: AccountIdentifier,
        staker: AccountIdentifier,
    ) -> Self {
        OperationMetadata {
            operator: Some(operator),
            staker: Some(staker),
            ..Default::default()
        }
    }

    pub fn add_delegated_stake(pool_address: AccountIdentifier, amount: Option<u64>) -> Self {
        OperationMetadata {
            pool_address: Some(pool_address),
            amount: amount.map(U64::from),
            ..Default::default()
        }
    }

    pub fn unlock_delegated_stake(pool_address: AccountIdentifier, amount: Option<u64>) -> Self {
        OperationMetadata {
            pool_address: Some(pool_address),
            amount: amount.map(U64::from),
            ..Default::default()
        }
    }

    pub fn withdraw_undelegated_stake(
        pool_address: AccountIdentifier,
        amount: Option<u64>,
    ) -> Self {
        OperationMetadata {
            pool_address: Some(pool_address),
            amount: amount.map(U64::from),
            ..Default::default()
        }
    }
}

/// Parses operations from the write set
///
/// This can only be done during a successful transaction because there are actual state changes.
/// It is more accurate because untracked scripts are included in balance operations
pub(crate) async fn parse_operations_from_write_set(
    server_context: &RosettaContext,
    struct_tag: &StructTag,
    address: AccountAddress,
    data: &[u8],
    events: &[ContractEvent],
    maybe_sender: Option<AccountAddress>,
    version: u64,
    operation_index: u64,
    changes: &WriteSet,
    object_to_owner: &mut HashMap<AccountAddress, AccountAddress>,
    store_to_currency: &mut HashMap<AccountAddress, Currency>,
) -> ApiResult<Vec<Operation>> {
    // Determine operation
    match (
        struct_tag.address,
        struct_tag.module.as_str(),
        struct_tag.name.as_str(),
        struct_tag.type_args.len(),
    ) {
        // TODO: Handle object transfer for transfer of fungible asset stores
        (AccountAddress::ONE, ACCOUNT_MODULE, ACCOUNT_RESOURCE, 0) => {
            parse_account_resource_changes(version, address, data, maybe_sender, operation_index)
        },
        (AccountAddress::ONE, STAKE_MODULE, STAKE_POOL_RESOURCE, 0) => {
            parse_stake_pool_resource_changes(
                server_context,
                version,
                address,
                data,
                events,
                operation_index,
            )
        },
        (AccountAddress::ONE, STAKING_CONTRACT_MODULE, STORE_RESOURCE, 0) => {
            parse_staking_contract_resource_changes(address, data, events, operation_index, changes)
                .await
        },
        (
            AccountAddress::ONE,
            STAKING_CONTRACT_MODULE,
            STAKING_GROUP_UPDATE_COMMISSION_RESOURCE,
            0,
        ) => parse_update_commission(address, data, events, operation_index, changes).await,
        (AccountAddress::ONE, DELEGATION_POOL_MODULE, DELEGATION_POOL_RESOURCE, 0) => {
            parse_delegation_pool_resource_changes(address, data, events, operation_index, changes)
                .await
        },
        (AccountAddress::ONE, COIN_MODULE, COIN_STORE_RESOURCE, 1) => {
            if let Some(type_tag) = struct_tag.type_args.first() {
                // Find the currency and parse it accordingly
                let maybe_currency = find_coin_currency(&server_context.currencies, type_tag);

                if let Some(currency) = maybe_currency {
                    parse_coinstore_changes(
                        currency.clone(),
                        type_tag.to_canonical_string(),
                        version,
                        address,
                        data,
                        events,
                        operation_index,
                    )
                } else {
                    Ok(vec![])
                }
            } else {
                warn!(
                    "Failed to parse coinstore {} at version {}",
                    struct_tag.to_canonical_string(),
                    version
                );
                Ok(vec![])
            }
        },
        (AccountAddress::ONE, FUNGIBLE_ASSET_MODULE, FUNGIBLE_STORE_RESOURCE, 0) => {
            parse_fungible_store_changes(
                object_to_owner,
                store_to_currency,
                address,
                events,
                operation_index,
            )
        },
        _ => {
            // Any unknown type will just skip the operations
            Ok(vec![])
        },
    }
}

fn parse_write_set<'a>(
    state_key: &'a StateKey,
    write_op: &'a WriteOp,
) -> Option<(StructTag, AccountAddress, &'a [u8])> {
    let (struct_tag, address) = match state_key.inner() {
        StateKeyInner::AccessPath(path) => match path.get_path() {
            Path::Resource(struct_tag) => (struct_tag, path.address),
            Path::ResourceGroup(group_tag) => (group_tag, path.address),
            _ => return None,
        },
        _ => {
            // Ignore all but access path
            return None;
        },
    };

    let bytes = write_op.bytes()?;

    Some((struct_tag, address, bytes))
}

pub(crate) fn preprocess_write_set<'a>(
    server_context: &RosettaContext,
    state_key: &'a StateKey,
    write_op: &'a WriteOp,
    _maybe_payload: Option<&TransactionPayload>,
    version: u64,
    object_to_owner: &mut HashMap<AccountAddress, AccountAddress>,
    store_to_currency: &mut HashMap<AccountAddress, Currency>,
) -> Vec<(StructTag, AccountAddress, Vec<u8>)> {
    let write_set_data = parse_write_set(state_key, write_op);
    if write_set_data.is_none() {
        return vec![];
    }
    let (struct_tag, address, data) = write_set_data.unwrap();

    // Determine owners of stores, and metadata addresses for stores
    let mut resources = vec![];
    match (
        struct_tag.address,
        struct_tag.module.as_str(),
        struct_tag.name.as_str(),
    ) {
        (AccountAddress::ONE, OBJECT_MODULE, OBJECT_RESOURCE_GROUP) => {
            // Parse the underlying resources in the group
            let maybe_resource_group = bcs::from_bytes::<ResourceGroup>(data);
            let resource_group = match maybe_resource_group {
                Ok(resource_group) => resource_group,
                Err(err) => {
                    warn!(
                        "Failed to parse object resource group in version {}: {:#}",
                        version, err
                    );
                    return vec![];
                },
            };

            for (struct_tag, bytes) in resource_group.iter() {
                match (
                    struct_tag.address,
                    struct_tag.module.as_str(),
                    struct_tag.name.as_str(),
                ) {
                    (AccountAddress::ONE, OBJECT_MODULE, OBJECT_CORE_RESOURCE) => {
                        parse_object_owner(address, bytes, object_to_owner);
                    },
                    (AccountAddress::ONE, FUNGIBLE_ASSET_MODULE, FUNGIBLE_STORE_RESOURCE) => {
                        parse_fungible_store_metadata(
                            &server_context.currencies,
                            version,
                            address,
                            bytes,
                            store_to_currency,
                        );
                    },
                    _ => {},
                }

                // Filter out transactions that are not framework
                if struct_tag.address == AccountAddress::ONE {
                    resources.push((struct_tag.clone(), address, bytes.clone()));
                }
            }
        },
        (AccountAddress::ONE, ..) => {
            // Filter out transactions that are not framework
            // TODO: maybe be more strict on what we filter
            resources.push((struct_tag.clone(), address, data.to_vec()));
        },
        _ => {},
    }

    resources
}

fn parse_object_owner(
    object_address: AccountAddress,
    data: &[u8],
    object_to_owner: &mut HashMap<AccountAddress, AccountAddress>,
) {
    if let Ok(object_core) = bcs::from_bytes::<ObjectCoreResource>(data) {
        object_to_owner.insert(object_address, object_core.owner);
    }
}

/// Parses any account resource changes, in this case only create account is supported
fn parse_account_resource_changes(
    version: u64,
    address: AccountAddress,
    data: &[u8],
    maybe_sender: Option<AccountAddress>,
    operation_index: u64,
) -> ApiResult<Vec<Operation>> {
    // TODO: Handle key rotation
    let mut operations = Vec::new();
    if let Ok(account) = bcs::from_bytes::<AccountResource>(data) {
        // Account sequence number increase (possibly creation)
        // Find out if it's the 0th sequence number (creation)
        if 0 == account.sequence_number() {
            operations.push(Operation::create_account(
                operation_index,
                Some(OperationStatusType::Success),
                address,
                maybe_sender.unwrap_or(AccountAddress::ONE),
            ));
        }
    } else {
        warn!(
            "Failed to parse AccountResource for {} at version {}",
            address, version
        );
    }

    Ok(operations)
}

fn parse_stake_pool_resource_changes(
    _server_context: &RosettaContext,
    _version: u64,
    _pool_address: AccountAddress,
    _data: &[u8],
    _events: &[ContractEvent],
    _operation_index: u64,
) -> ApiResult<Vec<Operation>> {
    let operations = Vec::new();

    // We at this point only care about balance changes from the stake pool
    // TODO: Balance changes are not supported for staking at this time
    /*    if let Some(owner_address) = server_context.pool_address_to_owner.get(&pool_address) {
            if let Ok(stakepool) = bcs::from_bytes::<StakePool>(data) {
                let total_stake_account = AccountIdentifier::total_stake_account(*owner_address);
                let operator_stake_account = AccountIdentifier::operator_stake_account(
                    *owner_address,
                    stakepool.operator_address,
                );

                // Retrieve add stake events
                let add_stake_events = filter_events(
                    events,
                    stakepool.add_stake_events.key(),
                    |event_key, event| {
                        if let Ok(event) = bcs::from_bytes::<aptos_types::stake_pool::AddStakeEvent>(
                            event.event_data(),
                        ) {
                            Some(event)
                        } else {
                            warn!(
                                "Failed to parse add stake event!  Skipping for {}:{}",
                                event_key.get_creator_address(),
                                event_key.get_creation_number()
                            );
                            None
                        }
                    },
                );

                // For every stake event, we distribute to the two sub balances.  The withdrawal from the account
                // is handled in coin
                for event in add_stake_events {
                    operations.push(Operation::deposit(
                        operation_index,
                        Some(OperationStatusType::Success),
                        total_stake_account.clone(),
                        native_coin(),
                        event.amount_added,
                    ));
                    operation_index += 1;
                    operations.push(Operation::deposit(
                        operation_index,
                        Some(OperationStatusType::Success),
                        operator_stake_account.clone(),
                        native_coin(),
                        event.amount_added,
                    ));
                    operation_index += 1;
                }

                // Retrieve withdraw stake events
                let withdraw_stake_events = filter_events(
                    events,
                    stakepool.withdraw_stake_events.key(),
                    |event_key, event| {
                        if let Ok(event) = bcs::from_bytes::<WithdrawStakeEvent>(event.event_data()) {
                            Some(event)
                        } else {
                            warn!(
                                "Failed to parse withdraw stake event!  Skipping for {}:{}",
                                event_key.get_creator_address(),
                                event_key.get_creation_number()
                            );
                            None
                        }
                    },
                );

                // For every withdraw event, we have to remove the amounts from the stake pools
                for event in withdraw_stake_events {
                    operations.push(Operation::withdraw(
                        operation_index,
                        Some(OperationStatusType::Success),
                        total_stake_account.clone(),
                        native_coin(),
                        event.amount_withdrawn,
                    ));
                    operation_index += 1;
                    operations.push(Operation::withdraw(
                        operation_index,
                        Some(OperationStatusType::Success),
                        operator_stake_account.clone(),
                        native_coin(),
                        event.amount_withdrawn,
                    ));
                    operation_index += 1;
                }

                // Retrieve staking rewards events
                let distribute_rewards_events = filter_events(
                    events,
                    stakepool.distribute_rewards_events.key(),
                    |event_key, event| {
                        if let Ok(event) = bcs::from_bytes::<DistributeRewardsEvent>(event.event_data())
                        {
                            Some(event)
                        } else {
                            warn!(
                                "Failed to parse distribute rewards event!  Skipping for {}:{}",
                                event_key.get_creator_address(),
                                event_key.get_creation_number()
                            );
                            None
                        }
                    },
                );

                // For every distribute rewards events, add to the staking pools
                for event in distribute_rewards_events {
                    operations.push(Operation::staking_reward(
                        operation_index,
                        Some(OperationStatusType::Success),
                        total_stake_account.clone(),
                        native_coin(),
                        event.rewards_amount,
                    ));
                    operation_index += 1;
                    operations.push(Operation::staking_reward(
                        operation_index,
                        Some(OperationStatusType::Success),
                        operator_stake_account.clone(),
                        native_coin(),
                        event.rewards_amount,
                    ));
                    operation_index += 1;
                }

                // Set voter has to be done at the `staking_contract` because there's no event for it here...

                // Handle set operator events
                let set_operator_events = filter_events(
                    events,
                    stakepool.set_operator_events.key(),
                    |event_key, event| {
                        if let Ok(event) = bcs::from_bytes::<aptos_types::stake_pool::SetOperatorEvent>(
                            event.event_data(),
                        ) {
                            Some(event)
                        } else {
                            // If we can't parse the withdraw event, then there's nothing
                            warn!(
                                "Failed to parse set operator event!  Skipping for {}:{}",
                                event_key.get_creator_address(),
                                event_key.get_creation_number()
                            );
                            None
                        }
                    },
                );

                // For every set operator event, change the operator, and transfer the money between them
                // We do this after balance transfers so the balance changes are easier
                let final_staked_amount = stakepool.get_total_staked_amount();
                for event in set_operator_events {
                    operations.push(Operation::set_operator(
                        operation_index,
                        Some(OperationStatusType::Success),
                        *owner_address,
                        Some(AccountIdentifier::base_account(event.old_operator)),
                        AccountIdentifier::base_account(event.new_operator),
                    ));
                    operation_index += 1;

                    let old_operator_account =
                        AccountIdentifier::operator_stake_account(*owner_address, event.old_operator);
                    operations.push(Operation::withdraw(
                        operation_index,
                        Some(OperationStatusType::Success),
                        old_operator_account,
                        native_coin(),
                        final_staked_amount,
                    ));
                    operation_index += 1;
                    let new_operator_account =
                        AccountIdentifier::operator_stake_account(*owner_address, event.old_operator);
                    operations.push(Operation::deposit(
                        operation_index,
                        Some(OperationStatusType::Success),
                        new_operator_account,
                        native_coin(),
                        final_staked_amount,
                    ));
                    operation_index += 1;
                }
            } else {
                warn!(
                    "Failed to parse stakepool for {} at version {}",
                    pool_address, version
                );
            }
        }
    */
    Ok(operations)
}

/// Handles 0x1::staking_contract resource changes
async fn parse_staking_contract_resource_changes(
    owner_address: AccountAddress,
    data: &[u8],
    events: &[ContractEvent],
    mut operation_index: u64,
    changes: &WriteSet,
) -> ApiResult<Vec<Operation>> {
    let mut operations = Vec::new();

    // This only handles the voter events from the staking contract
    // If there are direct events on the pool, they will be ignored
    if let Ok(store) = bcs::from_bytes::<Store>(data) {
        // Collect all the stake pools that were created
        let stake_pools: BTreeMap<AccountAddress, StakePool> = changes
            .write_op_iter()
            .filter_map(|(state_key, write_op)| {
                let data = write_op.bytes();

                let mut ret = None;
                if let (StateKeyInner::AccessPath(path), Some(data)) = (state_key.inner(), data) {
                    if let Some(struct_tag) = path.get_struct_tag() {
                        if let (AccountAddress::ONE, STAKE_MODULE, STAKE_POOL_RESOURCE) = (
                            struct_tag.address,
                            struct_tag.module.as_str(),
                            struct_tag.name.as_str(),
                        ) {
                            if let Ok(pool) = bcs::from_bytes::<StakePool>(data) {
                                ret = Some((path.address, pool))
                            }
                        }
                    }
                }

                ret
            })
            .collect();

        // Collect all operator events for all the stake pools, and add the total stake
        let mut set_operator_operations = vec![];
        let mut total_stake = 0;
        for (operator, staking_contract) in store.staking_contracts {
            if let Some(stake_pool) = stake_pools.get(&staking_contract.pool_address) {
                // Skip mismatched operators
                if operator != stake_pool.operator_address {
                    continue;
                }
                total_stake += stake_pool.get_total_staked_amount();

                // Get all set operator events for this stake pool
                let set_operator_events = filter_events(
                    events,
                    stake_pool.set_operator_events.key(),
                    |event_key, event| {
                        if let Ok(event) = bcs::from_bytes::<SetOperatorEvent>(event.event_data()) {
                            Some(event)
                        } else {
                            // If we can't parse the withdraw event, then there's nothing
                            warn!(
                                "Failed to parse set operator event!  Skipping for {}:{}",
                                event_key.get_creator_address(),
                                event_key.get_creation_number()
                            );
                            None
                        }
                    },
                );
                let set_operator_events_v2 =
                    filter_v2_events(&SET_OPERATOR_EVENT_TAG, events, |event| {
                        if let Ok(event) = bcs::from_bytes::<SetOperatorEvent>(event.event_data()) {
                            Some(event)
                        } else {
                            // If we can't parse the withdraw event, then there's nothing
                            warn!("Failed to parse set operator event!  Skipping",);
                            None
                        }
                    });

                for event in set_operator_events
                    .into_iter()
                    .chain(set_operator_events_v2)
                {
                    set_operator_operations.push(Operation::set_operator(
                        operation_index,
                        Some(OperationStatusType::Success),
                        owner_address,
                        Some(AccountIdentifier::base_account(event.old_operator)),
                        AccountIdentifier::base_account(event.new_operator),
                        None,
                    ));
                    operation_index += 1;
                }
            }
        }

        // Handle set voter events, there are no events on the stake pool
        let set_voter_events = filter_events(
            events,
            store.update_voter_events.key(),
            |event_key, event| {
                if let Ok(event) = bcs::from_bytes::<UpdateVoterEvent>(event.event_data()) {
                    Some(event)
                } else {
                    // If we can't parse the withdraw event, then there's nothing
                    warn!(
                        "Failed to parse update voter event!  Skipping for {}:{}",
                        event_key.get_creator_address(),
                        event_key.get_creation_number()
                    );
                    None
                }
            },
        );
        let set_voter_events_v2 = filter_v2_events(&UPDATE_VOTER_EVENT_TAG, events, |event| {
            if let Ok(event) = bcs::from_bytes::<UpdateVoterEvent>(event.event_data()) {
                Some(event)
            } else {
                // If we can't parse the withdraw event, then there's nothing
                warn!("Failed to parse update_voter event!  Skipping",);
                None
            }
        });

        // Parse all set voter events
        for event in set_voter_events.into_iter().chain(set_voter_events_v2) {
            operations.push(Operation::set_voter(
                operation_index,
                Some(OperationStatusType::Success),
                owner_address,
                Some(AccountIdentifier::base_account(event.operator)),
                AccountIdentifier::base_account(event.new_voter),
            ));
            operation_index += 1;
        }

        // Attach all set operators now, but with the total stake listed
        for mut operation in set_operator_operations.into_iter() {
            if let Some(inner) = operation.metadata.as_mut() {
                inner.staked_balance = Some(total_stake.into())
            }
            operations.push(operation);
        }

        // Handle distribute events, there are no events on the stake pool
        let distribute_staking_rewards_events =
            filter_events(events, store.distribute_events.key(), |event_key, event| {
                if let Ok(event) = bcs::from_bytes::<DistributeEvent>(event.event_data()) {
                    Some(event)
                } else {
                    // If we can't parse the withdraw event, then there's nothing
                    warn!(
                        "Failed to parse distribute event!  Skipping for {}:{}",
                        event_key.get_creator_address(),
                        event_key.get_creation_number()
                    );
                    None
                }
            });
        let distribute_staking_rewards_events_v2 =
            filter_v2_events(&DISTRIBUTE_STAKING_REWARDS_TAG, events, |event| {
                if let Ok(event) = bcs::from_bytes::<DistributeEvent>(event.event_data()) {
                    Some(event)
                } else {
                    // If we can't parse the withdraw event, then there's nothing
                    warn!("Failed to parse distribute_rewards event!  Skipping");
                    None
                }
            });

        // For every distribute events, add staking reward operation
        for event in distribute_staking_rewards_events
            .into_iter()
            .chain(distribute_staking_rewards_events_v2)
        {
            operations.push(Operation::staking_reward(
                operation_index,
                Some(OperationStatusType::Success),
                AccountIdentifier::base_account(event.recipient),
                native_coin(),
                event.amount,
            ));
            operation_index += 1;
        }
    }

    Ok(operations)
}

/// Parses 0x1::staking_contract commission updates
async fn parse_update_commission(
    _owner_address: AccountAddress,
    data: &[u8],
    events: &[ContractEvent],
    mut operation_index: u64,
    _changes: &WriteSet,
) -> ApiResult<Vec<Operation>> {
    let mut operations = Vec::new();

    // This only handles the voter events from the staking contract
    // If there are direct events on the pool, they will be ignored
    if let Ok(event_holder) = bcs::from_bytes::<StakingGroupUpdateCommissionEvent>(data) {
        let update_commission_events = filter_events(
            events,
            event_holder.update_commission_events.key(),
            |event_key, event| {
                if let Ok(event) = bcs::from_bytes::<UpdateCommissionEvent>(event.event_data()) {
                    Some(event)
                } else {
                    // If we can't parse the withdraw event, then there's nothing
                    warn!(
                        "Failed to parse update commission event!  Skipping for {}:{}",
                        event_key.get_creator_address(),
                        event_key.get_creation_number()
                    );
                    None
                }
            },
        );

        let update_commission_events_v2 =
            filter_v2_events(&UPDATE_COMMISSION_TAG, events, |event| {
                if let Ok(event) = bcs::from_bytes::<UpdateCommissionEvent>(event.event_data()) {
                    Some(event)
                } else {
                    // If we can't parse the withdraw event, then there's nothing
                    warn!("Failed to parse update commission event!  Skipping",);
                    None
                }
            });

        // For every distribute events, add staking reward operation
        for event in update_commission_events
            .into_iter()
            .chain(update_commission_events_v2)
        {
            operations.push(Operation::update_commission(
                operation_index,
                Some(OperationStatusType::Success),
                event.staker,
                Some(AccountIdentifier::base_account(event.operator)),
                Some(event.new_commission_percentage),
            ));
            operation_index += 1;
        }
    }
    Ok(operations)
}

/// Parses delegation pool changes to resources
async fn parse_delegation_pool_resource_changes(
    _owner_address: AccountAddress,
    _data: &[u8],
    events: &[ContractEvent],
    mut operation_index: u64,
    _changes: &WriteSet,
) -> ApiResult<Vec<Operation>> {
    let mut operations = vec![];

    for e in events {
        let struct_tag = match e.type_tag() {
            TypeTag::Struct(struct_tag) => struct_tag,
            _ => continue,
        };

        match (
            struct_tag.address,
            struct_tag.module.as_str(),
            struct_tag.name.as_str(),
        ) {
            (AccountAddress::ONE, DELEGATION_POOL_MODULE, WITHDRAW_STAKE_EVENT)
            | (AccountAddress::ONE, DELEGATION_POOL_MODULE, WITHDRAW_STAKE) => {
                let event: WithdrawUndelegatedEvent =
                    if let Ok(event) = bcs::from_bytes(e.event_data()) {
                        event
                    } else {
                        warn!(
                            "Failed to parse withdraw undelegated event! Skipping for {}:{}",
                            e.v1()?.key().get_creator_address(),
                            e.v1()?.key().get_creation_number()
                        );
                        continue;
                    };

                operations.push(Operation::withdraw_undelegated_stake(
                    operation_index,
                    Some(OperationStatusType::Success),
                    event.delegator_address,
                    AccountIdentifier::base_account(event.pool_address),
                    Some(event.amount_withdrawn),
                ));
                operation_index += 1;
            },
            _ => continue,
        }
    }

    Ok(operations)
}

/// Parses coin store direct changes, for withdraws and deposits
fn parse_coinstore_changes(
    currency: Currency,
    coin_type: String,
    version: u64,
    address: AccountAddress,
    data: &[u8],
    events: &[ContractEvent],
    mut operation_index: u64,
) -> ApiResult<Vec<Operation>> {
    let coin_store: CoinStoreResourceUntyped = if let Ok(coin_store) = bcs::from_bytes(data) {
        coin_store
    } else {
        warn!(
            "Coin store failed to parse for coin type {:?} and address {} at version {}",
            currency, address, version
        );
        return Ok(vec![]);
    };

    let mut operations = vec![];

    // Skip if there is no currency that can be found
    let mut withdraw_amounts = get_amount_from_event(events, coin_store.withdraw_events().key());
    withdraw_amounts.append(&mut get_amount_from_event_v2(
        events,
        &COIN_WITHDRAW_TYPE_TAG,
        address,
        &coin_type,
    ));
    for amount in withdraw_amounts {
        operations.push(Operation::withdraw(
            operation_index,
            Some(OperationStatusType::Success),
            AccountIdentifier::base_account(address),
            currency.clone(),
            amount,
        ));
        operation_index += 1;
    }

    let mut deposit_amounts = get_amount_from_event(events, coin_store.deposit_events().key());
    deposit_amounts.append(&mut get_amount_from_event_v2(
        events,
        &COIN_DEPOSIT_TYPE_TAG,
        address,
        &coin_type,
    ));
    for amount in deposit_amounts {
        operations.push(Operation::deposit(
            operation_index,
            Some(OperationStatusType::Success),
            AccountIdentifier::base_account(address),
            currency.clone(),
            amount,
        ));
        operation_index += 1;
    }

    Ok(operations)
}

fn parse_fungible_store_metadata(
    currencies: &HashSet<Currency>,
    version: u64,
    address: AccountAddress,
    data: &[u8],
    store_to_currency: &mut HashMap<AccountAddress, Currency>,
) {
    let fungible_store: FungibleStoreResource = if let Ok(fungible_store) = bcs::from_bytes(data) {
        fungible_store
    } else {
        warn!(
            "Fungible store failed to parse for address {} at version {} : {}",
            address,
            version,
            hex::encode(data)
        );
        return;
    };

    let metadata_address = fungible_store.metadata();
    let maybe_currency = find_fa_currency(currencies, metadata_address);
    if let Some(currency) = maybe_currency {
        store_to_currency.insert(address, currency);
    }
}

/// Parses fungible store direct changes, for withdraws and deposits
///
/// Note that, we don't know until we introspect the change, which fa it is
fn parse_fungible_store_changes(
    object_to_owner: &HashMap<AccountAddress, AccountAddress>,
    store_to_currency: &HashMap<AccountAddress, Currency>,
    address: AccountAddress,
    events: &[ContractEvent],
    mut operation_index: u64,
) -> ApiResult<Vec<Operation>> {
    let mut operations = vec![];

    // Find the fungible asset currency association
    let maybe_currency = store_to_currency.get(&address);
    if maybe_currency.is_none() {
        return Ok(operations);
    }
    let currency = maybe_currency.unwrap();

    // If there's a currency, let's fill in operations
    // If we don't have an owner here, there's missing data on the writeset
    let maybe_owner = object_to_owner.get(&address);
    if maybe_owner.is_none() {
        warn!(
            "First pass did not catch owner for fungible store \"{}\", returning no operations",
            address
        );
        return Ok(operations);
    }

    let owner = maybe_owner.copied().unwrap();

    // We double check if the address is a primary store
    let account = if is_primary_store(&address, &owner, currency) {
        AccountIdentifier::base_account(owner)
    } else {
        AccountIdentifier::secondary_store_account(&address, currency)
    };

    let withdraw_amounts = get_amount_from_fa_event(events, &WITHDRAW_TYPE_TAG, address);
    for amount in withdraw_amounts {
        operations.push(Operation::withdraw(
            operation_index,
            Some(OperationStatusType::Success),
            account.clone(),
            currency.clone(),
            amount,
        ));
        operation_index += 1;
    }

    let deposit_amounts = get_amount_from_fa_event(events, &DEPOSIT_TYPE_TAG, address);
    for amount in deposit_amounts {
        operations.push(Operation::deposit(
            operation_index,
            Some(OperationStatusType::Success),
            account.clone(),
            currency.clone(),
            amount,
        ));
        operation_index += 1;
    }

    Ok(operations)
}

fn is_primary_store(
    store_address: &AccountAddress,
    owner: &AccountAddress,
    currency: &Currency,
) -> bool {
    let metadata_address = match currency.metadata.as_ref().and_then(|metadata| {
        metadata
            .fa_address
            .as_ref()
            .map(|fa_address| AccountAddress::from_str(fa_address))
    }) {
        Some(Ok(metadata_address)) => metadata_address,
        Some(Err(_)) => return false,
        None if currency == &native_coin() => AccountAddress::TEN,
        None => return false,
    };

    create_derived_object_address(*owner, metadata_address) == *store_address
}

/// Pulls the balance change from a withdraw or deposit event
fn get_amount_from_event(events: &[ContractEvent], event_key: &EventKey) -> Vec<u64> {
    filter_events(events, event_key, |event_key, event| {
        if let Ok(event) = bcs::from_bytes::<WithdrawEvent>(event.event_data()) {
            Some(event.amount())
        } else {
            // If we can't parse the withdraw event, then there's nothing
            warn!(
                "Failed to parse coin store withdraw event!  Skipping for {}:{}",
                event_key.get_creator_address(),
                event_key.get_creation_number()
            );
            None
        }
    })
}

fn get_amount_from_event_v2(
    events: &[ContractEvent],
    type_tag: &TypeTag,
    account_address: AccountAddress,
    coin_type: &String,
) -> Vec<u64> {
    filter_v2_events(type_tag, events, |event| {
        if let Ok(event) = bcs::from_bytes::<CoinWithdraw>(event.event_data()) {
            if event.account() == &account_address && event.coin_type() == coin_type {
                Some(event.amount())
            } else {
                None
            }
        } else {
            // If we can't parse the withdraw event, then there's nothing
            warn!("Failed to parse fungible store event!  Skipping");
            None
        }
    })
}

/// Pulls the balance change from a withdraw or deposit event
fn get_amount_from_fa_event(
    events: &[ContractEvent],
    type_tag: &TypeTag,
    store_address: AccountAddress,
) -> Vec<u64> {
    filter_v2_events(type_tag, events, |event| {
        // since we are only deserializing, both DepositFAEvent and WithdrawFAEvent have identical fields
        if let Ok(event) = bcs::from_bytes::<DepositFAEvent>(event.event_data()) {
            if event.store == store_address {
                Some(event.amount)
            } else {
                None
            }
        } else {
            // If we can't parse the withdraw event, then there's nothing
            warn!("Failed to parse fungible store event!  Skipping");
            None
        }
    })
}

/// Filter v2 FeeStatement events with non-zero storage_fee_refund
pub(crate) fn get_fee_statement_from_event(events: &[ContractEvent]) -> Vec<FeeStatement> {
    events
        .iter()
        .filter_map(|event| {
            if let Ok(Some(fee_statement)) = event.try_v2_typed(&FEE_STATEMENT_EVENT_TYPE) {
                Some(fee_statement)
            } else {
                None
            }
        })
        .collect()
}

/// Filters events given a specific event key
fn filter_events<F: Fn(&EventKey, &ContractEvent) -> Option<T>, T>(
    events: &[ContractEvent],
    event_key: &EventKey,
    parser: F,
) -> Vec<T> {
    events
        .iter()
        .filter(|event| event.is_v1())
        .filter(|event| event.v1().unwrap().key() == event_key)
        .sorted_by(|a, b| {
            a.v1()
                .unwrap()
                .sequence_number()
                .cmp(&b.v1().unwrap().sequence_number())
        })
        .filter_map(|event| parser(event_key, event))
        .collect()
}

fn filter_v2_events<F: Fn(&ContractEventV2) -> Option<T>, T>(
    event_type: &TypeTag,
    events: &[ContractEvent],
    parser: F,
) -> Vec<T> {
    events
        .iter()
        .filter(|event| event.is_v2())
        .map(|event| event.v2().unwrap())
        .filter(|event| event_type == event.type_tag())
        .filter_map(parser)
        .collect()
}

/// An enum for processing which operation is in a transaction
pub enum OperationDetails {
    CreateAccount,
    TransferCoin {
        currency: Currency,
        withdraw_event_key: Option<EventKey>,
        deposit_event_key: Option<EventKey>,
    },
}
