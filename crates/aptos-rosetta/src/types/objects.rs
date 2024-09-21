// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Objects of the Rosetta spec
//!
//! [Spec](https://www.rosetta-api.org/docs/api_objects.html)

use crate::{
    common::{is_native_coin, native_coin, native_coin_tag},
    construction::{
        parse_create_stake_pool_operation, parse_delegation_pool_add_stake_operation,
        parse_delegation_pool_unlock_operation, parse_delegation_pool_withdraw_operation,
        parse_distribute_staking_rewards_operation, parse_reset_lockup_operation,
        parse_set_operator_operation, parse_set_voter_operation, parse_unlock_stake_operation,
        parse_update_commission_operation,
    },
    error::ApiResult,
    types::{
        move_types::*, AccountIdentifier, BlockIdentifier, Error, OperationIdentifier,
        OperationStatus, OperationStatusType, OperationType, TransactionIdentifier,
    },
    ApiError, RosettaContext,
};
use anyhow::anyhow;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PublicKey, ValidCryptoMaterialStringExt};
use aptos_logger::warn;
use aptos_rest_client::aptos_api_types::{TransactionOnChainData, U64};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, CoinStoreResourceUntyped, WithdrawEvent},
    contract_event::{ContractEvent, FEE_STATEMENT_EVENT_TYPE},
    event::EventKey,
    fee_statement::FeeStatement,
    stake_pool::{SetOperatorEvent, StakePool},
    state_store::state_key::{inner::StateKeyInner, StateKey},
    transaction::{EntryFunction, TransactionPayload},
    write_set::{WriteOp, WriteSet},
};
use itertools::Itertools;
use move_core_types::language_storage::TypeTag;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    fmt::{Display, Formatter},
    hash::Hash,
    str::FromStr,
};

/// A description of all types used by the Rosetta implementation.
///
/// This is used to verify correctness of the implementation and to check things like
/// operation names, and error names.
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Allow.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Allow {
    /// List of all possible operation statuses
    pub operation_statuses: Vec<OperationStatus>,
    /// List of all possible writeset types
    pub operation_types: Vec<String>,
    /// List of all possible errors
    pub errors: Vec<Error>,
    /// If the server is allowed to lookup historical transactions
    pub historical_balance_lookup: bool,
    /// All times after this are valid timestamps
    pub timestamp_start_index: u64,
    /// All call methods supported
    pub call_methods: Vec<String>,
    /// A list of balance exemptions.  These should be as minimal as possible, otherwise it becomes
    /// more complicated for users
    pub balance_exemptions: Vec<BalanceExemption>,
    /// Determines if mempool can change the balance on an account
    /// This should be set to false
    pub mempool_coins: bool,
}

/// Amount of a [`Currency`] in atomic units
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Amount.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Amount {
    /// Value of transaction as a String representation of an integer
    pub value: String,
    /// [`Currency`]
    pub currency: Currency,
}

impl Amount {
    pub fn suggested_gas_fee(gas_unit_price: u64, max_gas_amount: u64) -> Amount {
        Amount {
            value: (gas_unit_price * max_gas_amount).to_string(),
            currency: native_coin(),
        }
    }

    pub fn value(&self) -> ApiResult<i128> {
        i128::from_str(&self.value)
            .map_err(|_| ApiError::InvalidTransferOperations(Some("Withdraw amount is invalid")))
    }
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BalanceExemption.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BalanceExemption {}

/// Representation of a Block for a blockchain.
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Block.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Block {
    /// Block identifier of the current block
    pub block_identifier: BlockIdentifier,
    /// Block identifier of the previous block
    pub parent_block_identifier: BlockIdentifier,
    /// Timestamp in milliseconds to the block from the UNIX_EPOCH
    pub timestamp: u64,
    /// Transactions associated with the version.  In aptos there should only be one transaction
    pub transactions: Vec<Transaction>,
}

/// A combination of a transaction and the block associated.  In Aptos, this is just the same
/// as the version associated with the transaction
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockTransaction.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockTransaction {
    /// Block associated with transaction
    block_identifier: BlockIdentifier,
    /// Transaction associated with block
    transaction: Transaction,
}

/// Currency represented as atomic units including decimals
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Currency.html)
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Currency {
    /// Symbol of currency
    pub symbol: String,
    /// Number of decimals to be considered in the currency
    pub decimals: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<CurrencyMetadata>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CurrencyMetadata {
    pub move_type: String,
}

/// Various signing curves supported by Rosetta.  We only use [`CurveType::Edwards25519`]
/// [API Spec](https://www.rosetta-api.org/docs/models/CurveType.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveType {
    Edwards25519,
}

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

/// Public key used for the rosetta implementation.  All private keys will never be handled
/// in the Rosetta implementation.
///
/// [API Spec](https://www.rosetta-api.org/docs/models/PublicKey.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PublicKey {
    /// Hex encoded public key bytes
    pub hex_bytes: String,
    /// Curve type associated with the key
    pub curve_type: CurveType,
}

impl TryFrom<Ed25519PublicKey> for PublicKey {
    type Error = anyhow::Error;

    fn try_from(public_key: Ed25519PublicKey) -> Result<Self, Self::Error> {
        Ok(PublicKey {
            hex_bytes: public_key.to_encoded_string()?,
            curve_type: CurveType::Edwards25519,
        })
    }
}

impl TryFrom<PublicKey> for Ed25519PublicKey {
    type Error = anyhow::Error;

    fn try_from(public_key: PublicKey) -> Result<Self, Self::Error> {
        if public_key.curve_type != CurveType::Edwards25519 {
            return Err(anyhow!("Invalid curve type"));
        }

        Ok(Ed25519PublicKey::from_encoded_string(
            &public_key.hex_bytes,
        )?)
    }
}

/// Signature containing the signed payload and the encoded signed payload
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Signature.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Signature {
    /// Payload to be signed
    pub signing_payload: SigningPayload,
    /// Public key related to the signature
    pub public_key: PublicKey,
    /// Cryptographic signature type
    pub signature_type: SignatureType,
    /// Hex bytes of the signature
    pub hex_bytes: String,
}

/// Cryptographic signature type used for signing transactions.  Aptos only uses
/// [`SignatureType::Ed25519`]
///
/// [API Spec](https://www.rosetta-api.org/docs/models/SignatureType.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureType {
    Ed25519,
}

/// Signing payload should be signed by the client with their own private key
///
/// [API Spec](https://www.rosetta-api.org/docs/models/SigningPayload.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SigningPayload {
    /// Account identifier of the signer
    pub account_identifier: AccountIdentifier,
    /// Hex encoded string of payload bytes to be signed
    pub hex_bytes: String,
    /// Signature type to sign with
    pub signature_type: Option<SignatureType>,
}

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

        // Operations must be sequential and operation index must always be in the same order
        // with no gaps
        let successful = txn_info.status().is_success();
        let mut operations = vec![];
        let mut operation_index: u64 = 0;
        if successful {
            // Parse all operations from the writeset changes in a success
            for (state_key, write_op) in &txn.changes {
                let mut ops = parse_operations_from_write_set(
                    server_context,
                    state_key,
                    write_op,
                    &events,
                    maybe_user_txn.map(|inner| inner.sender()),
                    maybe_user_txn.map(|inner| inner.payload()),
                    txn.version,
                    operation_index,
                    &txn.changes,
                )
                .await?;
                operation_index += ops.len() as u64;
                operations.append(&mut ops);
            }

            // For storage fee refund
            if let Some(user_txn) = maybe_user_txn {
                let fee_events = get_fee_statement_from_event(&events);
                for event in fee_events {
                    operations.push(Operation::deposit(
                        operation_index,
                        Some(OperationStatusType::Success),
                        AccountIdentifier::base_account(user_txn.sender()),
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
        if let Some(txn) = maybe_user_txn {
            operations.push(Operation::gas_fee(
                operation_index,
                txn.sender(),
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
            (AccountAddress::ONE, COIN_MODULE, TRANSFER_FUNCTION) => {
                // Only put the transfer in if we can understand the currency
                if let Some(type_tag) = inner.ty_args().first() {
                    // We don't want to do lookups on failures for currencies that don't exist,
                    // so we only look up cached info not new info
                    // TODO: If other coins are supported, this will need to be updated to handle more coins
                    if type_tag == &native_coin_tag() {
                        operations = parse_transfer_from_txn_payload(
                            inner,
                            native_coin(),
                            sender,
                            operation_index,
                        )
                    }
                }
            },
            (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNCTION) => {
                // We could add a create here as well, but we don't know if it will actually happen
                operations =
                    parse_transfer_from_txn_payload(inner, native_coin(), sender, operation_index)
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
fn parse_transfer_from_txn_payload(
    payload: &EntryFunction,
    currency: Currency,
    sender: AccountAddress,
    operation_index: u64,
) -> Vec<Operation> {
    let mut operations = vec![];

    let args = payload.args();
    let maybe_receiver = args
        .first()
        .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded));
    let maybe_amount = args.get(1).map(|encoded| bcs::from_bytes::<u64>(encoded));

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

/// Parses operations from the write set
///
/// This can only be done during a successful transaction because there are actual state changes.
/// It is more accurate because untracked scripts are included in balance operations
async fn parse_operations_from_write_set(
    server_context: &RosettaContext,
    state_key: &StateKey,
    write_op: &WriteOp,
    events: &[ContractEvent],
    maybe_sender: Option<AccountAddress>,
    _maybe_payload: Option<&TransactionPayload>,
    version: u64,
    operation_index: u64,
    changes: &WriteSet,
) -> ApiResult<Vec<Operation>> {
    let (struct_tag, address) = match state_key.inner() {
        StateKeyInner::AccessPath(path) => {
            if let Some(struct_tag) = path.get_struct_tag() {
                (struct_tag, path.address)
            } else {
                return Ok(vec![]);
            }
        },
        _ => {
            // Ignore all but access path
            return Ok(vec![]);
        },
    };

    let bytes = match write_op.bytes() {
        Some(bytes) => bytes,
        None => return Ok(vec![]),
    };
    let data = &bytes;

    // Determine operation
    match (
        struct_tag.address,
        struct_tag.module.as_str(),
        struct_tag.name.as_str(),
        struct_tag.type_args.len(),
    ) {
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
                // TODO: This will need to be updated to support more coins
                if type_tag == &native_coin_tag() {
                    parse_coinstore_changes(
                        native_coin(),
                        version,
                        address,
                        data,
                        events,
                        operation_index,
                    )
                    .await
                } else {
                    Ok(vec![])
                }
            } else {
                warn!(
                    "Failed to parse coinstore {} at version {}",
                    struct_tag, version
                );
                Ok(vec![])
            }
        },
        _ => {
            // Any unknown type will just skip the operations
            Ok(vec![])
        },
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
            .iter()
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

                for event in set_operator_events.iter() {
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

        // Parse all set voter events
        for event in set_voter_events {
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

        // For every distribute events, add staking reward operation
        for event in distribute_staking_rewards_events {
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

        // For every distribute events, add staking reward operation
        for event in update_commission_events {
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
            (AccountAddress::ONE, DELEGATION_POOL_MODULE, WITHDRAW_STAKE_EVENT) => {
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
async fn parse_coinstore_changes(
    currency: Currency,
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

    // TODO: Handle Event V2 here for migration from Event V1

    // Skip if there is no currency that can be found
    let withdraw_amounts = get_amount_from_event(events, coin_store.withdraw_events().key());
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

    let deposit_amounts = get_amount_from_event(events, coin_store.deposit_events().key());
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

/// Filter v2 FeeStatement events with non-zero storage_fee_refund
fn get_fee_statement_from_event(events: &[ContractEvent]) -> Vec<FeeStatement> {
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

/// An enum for processing which operation is in a transaction
pub enum OperationDetails {
    CreateAccount,
    TransferCoin {
        currency: Currency,
        withdraw_event_key: Option<EventKey>,
        deposit_event_key: Option<EventKey>,
    },
}

/// A holder for all information related to a specific transaction
/// built from [`Operation`]s
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InternalOperation {
    CreateAccount(CreateAccount),
    Transfer(Transfer),
    SetOperator(SetOperator),
    SetVoter(SetVoter),
    InitializeStakePool(InitializeStakePool),
    ResetLockup(ResetLockup),
    UnlockStake(UnlockStake),
    UpdateCommission(UpdateCommission),
    WithdrawUndelegated(WithdrawUndelegated),
    DistributeStakingRewards(DistributeStakingRewards),
    AddDelegatedStake(AddDelegatedStake),
    UnlockDelegatedStake(UnlockDelegatedStake),
}

impl InternalOperation {
    /// Pulls the [`InternalOperation`] from the set of [`Operation`]
    /// TODO: this needs to be broken up
    pub fn extract(operations: &Vec<Operation>) -> ApiResult<InternalOperation> {
        match operations.len() {
            // Single operation actions
            1 => {
                if let Some(operation) = operations.first() {
                    match OperationType::from_str(&operation.operation_type) {
                        Ok(OperationType::InitializeStakePool) => {
                            if let (
                                Some(OperationMetadata {
                                    new_operator,
                                    new_voter,
                                    staked_balance,
                                    commission_percentage,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let owner_address = account.account_address()?;
                                let operator_address = if let Some(address) = new_operator {
                                    address.account_address()?
                                } else {
                                    owner_address
                                };
                                let voter_address = if let Some(address) = new_voter {
                                    address.account_address()?
                                } else {
                                    owner_address
                                };

                                return Ok(Self::InitializeStakePool(InitializeStakePool {
                                    owner: owner_address,
                                    operator: operator_address,
                                    voter: voter_address,
                                    amount: staked_balance.map(u64::from).unwrap_or_default(),
                                    commission_percentage: commission_percentage
                                        .map(u64::from)
                                        .unwrap_or_default(),
                                    seed: vec![],
                                }));
                            }
                        },
                        Ok(OperationType::CreateAccount) => {
                            if let (
                                Some(OperationMetadata {
                                    sender: Some(sender),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::CreateAccount(CreateAccount {
                                    sender: sender.account_address()?,
                                    new_account: account.account_address()?,
                                }));
                            }
                        },
                        Ok(OperationType::SetOperator) => {
                            if let (
                                Some(OperationMetadata {
                                    old_operator,
                                    new_operator: Some(new_operator),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let old_operator = if let Some(old_operator) = old_operator {
                                    Some(old_operator.account_address()?)
                                } else {
                                    None
                                };

                                return Ok(Self::SetOperator(SetOperator {
                                    owner: account.account_address()?,
                                    old_operator,
                                    new_operator: new_operator.account_address()?,
                                }));
                            }
                        },
                        Ok(OperationType::SetVoter) => {
                            if let (
                                Some(OperationMetadata {
                                    operator,
                                    new_voter: Some(new_voter),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    Some(operator.account_address()?)
                                } else {
                                    None
                                };
                                return Ok(Self::SetVoter(SetVoter {
                                    owner: account.account_address()?,
                                    operator,
                                    new_voter: new_voter.account_address()?,
                                }));
                            }
                        },
                        Ok(OperationType::ResetLockup) => {
                            if let (Some(OperationMetadata { operator, .. }), Some(account)) =
                                (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    operator.account_address()?
                                } else {
                                    return Err(ApiError::InvalidInput(Some(
                                        "Reset lockup missing operator field".to_string(),
                                    )));
                                };
                                return Ok(Self::ResetLockup(ResetLockup {
                                    owner: account.account_address()?,
                                    operator,
                                }));
                            }
                        },
                        Ok(OperationType::UnlockStake) => {
                            if let (
                                Some(OperationMetadata {
                                    operator, amount, ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    operator.account_address()?
                                } else {
                                    return Err(ApiError::InvalidInput(Some(
                                        "Unlock Stake missing operator field".to_string(),
                                    )));
                                };
                                return Ok(Self::UnlockStake(UnlockStake {
                                    owner: account.account_address()?,
                                    operator,
                                    amount: amount.map(u64::from).unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::UpdateCommission) => {
                            if let (
                                Some(OperationMetadata {
                                    operator,
                                    commission_percentage,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    operator.account_address()?
                                } else {
                                    return Err(ApiError::InvalidInput(Some(
                                        "Unlock Stake missing operator field".to_string(),
                                    )));
                                };
                                return Ok(Self::UpdateCommission(UpdateCommission {
                                    owner: account.account_address()?,
                                    operator,
                                    new_commission_percentage: commission_percentage
                                        .map(u64::from)
                                        .unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::DistributeStakingRewards) => {
                            if let (
                                Some(OperationMetadata {
                                    operator: Some(operator),
                                    staker: Some(staker),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::DistributeStakingRewards(
                                    DistributeStakingRewards {
                                        sender: account.account_address()?,
                                        operator: operator.account_address()?,
                                        staker: staker.account_address()?,
                                    },
                                ));
                            }
                        },
                        Ok(OperationType::AddDelegatedStake) => {
                            if let (
                                Some(OperationMetadata {
                                    pool_address: Some(pool_address),
                                    amount,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::AddDelegatedStake(AddDelegatedStake {
                                    delegator: account.account_address()?,
                                    pool_address: pool_address.account_address()?,
                                    amount: amount.map(u64::from).unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::UnlockDelegatedStake) => {
                            if let (
                                Some(OperationMetadata {
                                    pool_address: Some(pool_address),
                                    amount,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::UnlockDelegatedStake(UnlockDelegatedStake {
                                    delegator: account.account_address()?,
                                    pool_address: pool_address.account_address()?,
                                    amount: amount.map(u64::from).unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::WithdrawUndelegatedFunds) => {
                            if let (
                                Some(OperationMetadata {
                                    pool_address: Some(pool_address),
                                    amount,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::WithdrawUndelegated(WithdrawUndelegated {
                                    delegator: account.account_address()?,
                                    amount_withdrawn: amount.map(u64::from).unwrap_or_default(),
                                    pool_address: pool_address.account_address()?,
                                }));
                            }
                        },
                        _ => {},
                    }
                }

                // Return invalid operations if for any reason parsing fails
                Err(ApiError::InvalidOperations(Some(format!(
                    "Unrecognized single operation {:?}",
                    operations
                ))))
            },
            // Double operation actions (only coin transfer)
            2 => Ok(Self::Transfer(Transfer::extract_transfer(operations)?)),
            // Anything else is not expected
            _ => Err(ApiError::InvalidOperations(Some(format!(
                "Unrecognized operation combination {:?}",
                operations
            )))),
        }
    }

    /// The sender of the transaction
    pub fn sender(&self) -> AccountAddress {
        match self {
            Self::CreateAccount(inner) => inner.sender,
            Self::Transfer(inner) => inner.sender,
            Self::SetOperator(inner) => inner.owner,
            Self::SetVoter(inner) => inner.owner,
            Self::InitializeStakePool(inner) => inner.owner,
            Self::ResetLockup(inner) => inner.owner,
            Self::UnlockStake(inner) => inner.owner,
            Self::UpdateCommission(inner) => inner.owner,
            Self::WithdrawUndelegated(inner) => inner.delegator,
            Self::DistributeStakingRewards(inner) => inner.sender,
            Self::AddDelegatedStake(inner) => inner.delegator,
            Self::UnlockDelegatedStake(inner) => inner.delegator,
        }
    }

    pub fn payload(
        &self,
    ) -> ApiResult<(aptos_types::transaction::TransactionPayload, AccountAddress)> {
        Ok(match self {
            InternalOperation::CreateAccount(create_account) => (
                aptos_stdlib::aptos_account_create_account(create_account.new_account),
                create_account.sender,
            ),
            InternalOperation::Transfer(transfer) => {
                is_native_coin(&transfer.currency)?;
                (
                    aptos_stdlib::aptos_account_transfer(transfer.receiver, transfer.amount.0),
                    transfer.sender,
                )
            },
            InternalOperation::SetOperator(set_operator) => {
                if set_operator.old_operator.is_none() {
                    return Err(ApiError::InvalidInput(Some(
                        "SetOperator doesn't have an old operator".to_string(),
                    )));
                }
                (
                    aptos_stdlib::staking_contract_switch_operator_with_same_commission(
                        set_operator.old_operator.unwrap(),
                        set_operator.new_operator,
                    ),
                    set_operator.owner,
                )
            },
            InternalOperation::SetVoter(set_voter) => {
                if set_voter.operator.is_none() {
                    return Err(ApiError::InvalidInput(Some(
                        "Set voter doesn't have an operator".to_string(),
                    )));
                }
                (
                    aptos_stdlib::staking_contract_update_voter(
                        set_voter.operator.unwrap(),
                        set_voter.new_voter,
                    ),
                    set_voter.owner,
                )
            },
            InternalOperation::InitializeStakePool(init_stake_pool) => (
                aptos_stdlib::staking_contract_create_staking_contract(
                    init_stake_pool.operator,
                    init_stake_pool.voter,
                    init_stake_pool.amount,
                    init_stake_pool.commission_percentage,
                    init_stake_pool.seed.clone(),
                ),
                init_stake_pool.owner,
            ),
            InternalOperation::ResetLockup(reset_lockup) => (
                aptos_stdlib::staking_contract_reset_lockup(reset_lockup.operator),
                reset_lockup.owner,
            ),
            InternalOperation::UnlockStake(unlock_stake) => (
                aptos_stdlib::staking_contract_unlock_stake(
                    unlock_stake.operator,
                    unlock_stake.amount,
                ),
                unlock_stake.owner,
            ),
            InternalOperation::UpdateCommission(update_commision) => (
                aptos_stdlib::staking_contract_update_commision(
                    update_commision.operator,
                    update_commision.new_commission_percentage,
                ),
                update_commision.owner,
            ),
            InternalOperation::DistributeStakingRewards(distribute_staking_rewards) => (
                aptos_stdlib::staking_contract_distribute(
                    distribute_staking_rewards.staker,
                    distribute_staking_rewards.operator,
                ),
                distribute_staking_rewards.sender,
            ),
            InternalOperation::AddDelegatedStake(add_delegated_stake) => (
                aptos_stdlib::delegation_pool_add_stake(
                    add_delegated_stake.pool_address,
                    add_delegated_stake.amount,
                ),
                add_delegated_stake.delegator,
            ),
            InternalOperation::UnlockDelegatedStake(unlock_delegated_stake) => (
                aptos_stdlib::delegation_pool_unlock(
                    unlock_delegated_stake.pool_address,
                    unlock_delegated_stake.amount,
                ),
                unlock_delegated_stake.delegator,
            ),
            InternalOperation::WithdrawUndelegated(withdraw_undelegated) => (
                aptos_stdlib::delegation_pool_withdraw(
                    withdraw_undelegated.pool_address,
                    withdraw_undelegated.amount_withdrawn,
                ),
                withdraw_undelegated.delegator,
            ),
        })
    }
}

/// Operation to create an account
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateAccount {
    pub sender: AccountAddress,
    pub new_account: AccountAddress,
}

/// Operation to transfer coins between accounts
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transfer {
    pub sender: AccountAddress,
    pub receiver: AccountAddress,
    pub amount: U64,
    pub currency: Currency,
}

impl Transfer {
    pub fn extract_transfer(operations: &Vec<Operation>) -> ApiResult<Transfer> {
        // Only support 1:1 P2P transfer
        // This is composed of a Deposit and a Withdraw operation
        if operations.len() != 2 {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have exactly 2 operations a withdraw and a deposit",
            )));
        }

        let mut op_map = HashMap::new();
        for op in operations {
            let op_type = OperationType::from_str(&op.operation_type)?;
            op_map.insert(op_type, op);
        }

        if !op_map.contains_key(&OperationType::Deposit) {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have a deposit",
            )));
        }

        // Verify accounts and amounts
        let (sender, withdraw_amount) = if let Some(withdraw) = op_map.get(&OperationType::Withdraw)
        {
            if let (Some(account), Some(amount)) = (&withdraw.account, &withdraw.amount) {
                if account.is_base_account() {
                    (account.account_address()?, amount)
                } else {
                    return Err(ApiError::InvalidInput(Some(
                        "Transferring stake amounts is not supported".to_string(),
                    )));
                }
            } else {
                return Err(ApiError::InvalidTransferOperations(Some(
                    "Invalid withdraw account provided",
                )));
            }
        } else {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have a withdraw",
            )));
        };

        let (receiver, deposit_amount) = if let Some(deposit) = op_map.get(&OperationType::Deposit)
        {
            if let (Some(account), Some(amount)) = (&deposit.account, &deposit.amount) {
                if account.is_base_account() {
                    (account.account_address()?, amount)
                } else {
                    return Err(ApiError::InvalidInput(Some(
                        "Transferring stake amounts is not supported".to_string(),
                    )));
                }
            } else {
                return Err(ApiError::InvalidTransferOperations(Some(
                    "Invalid deposit account provided",
                )));
            }
        } else {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have a deposit",
            )));
        };

        // Currencies have to be the same
        if withdraw_amount.currency != deposit_amount.currency {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Currency mismatch between withdraw and deposit",
            )));
        }

        // Check that the currency is supported
        // TODO: in future use currency, since there's more than just 1
        is_native_coin(&withdraw_amount.currency)?;

        let withdraw_value = i128::from_str(&withdraw_amount.value)
            .map_err(|_| ApiError::InvalidTransferOperations(Some("Withdraw amount is invalid")))?;
        let deposit_value = i128::from_str(&deposit_amount.value)
            .map_err(|_| ApiError::InvalidTransferOperations(Some("Deposit amount is invalid")))?;

        // We can't create or destroy coins, they must be negatives of each other
        if -withdraw_value != deposit_value {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Withdraw amount must be equal to negative of deposit amount",
            )));
        }

        // We converted to u128 to ensure no loss of precision in comparison,
        // but now we actually have to check it's a u64
        if deposit_value > u64::MAX as i128 {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Transfer amount must not be greater than u64 max",
            )));
        }

        let transfer_amount = deposit_value as u64;

        Ok(Transfer {
            sender,
            receiver,
            amount: transfer_amount.into(),
            currency: deposit_amount.currency.clone(),
        })
    }
}

/// Set operator
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetOperator {
    pub owner: AccountAddress,
    pub old_operator: Option<AccountAddress>,
    pub new_operator: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetVoter {
    pub owner: AccountAddress,
    pub operator: Option<AccountAddress>,
    pub new_voter: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InitializeStakePool {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
    pub voter: AccountAddress,
    pub amount: u64,
    pub commission_percentage: u64,
    pub seed: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResetLockup {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnlockStake {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
    pub amount: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UpdateCommission {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
    pub new_commission_percentage: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WithdrawUndelegated {
    pub delegator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount_withdrawn: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DistributeStakingRewards {
    pub sender: AccountAddress,
    pub operator: AccountAddress,
    pub staker: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AddDelegatedStake {
    pub delegator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnlockDelegatedStake {
    pub delegator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount: u64,
}
