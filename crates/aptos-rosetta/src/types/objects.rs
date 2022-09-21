// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Objects of the Rosetta spec
//!
//! [Spec](https://www.rosetta-api.org/docs/api_objects.html)

use crate::types::{
    ACCOUNT_MODULE, ACCOUNT_RESOURCE, APTOS_ACCOUNT_MODULE, COIN_MODULE, COIN_STORE_RESOURCE,
    CREATE_ACCOUNT_FUNCTION, SET_OPERATOR_FUNCTION, SET_VOTER_FUNCTION, STAKE_MODULE,
    STAKE_POOL_RESOURCE, TRANSFER_FUNCTION,
};
use crate::{
    common::{is_native_coin, native_coin},
    error::ApiResult,
    types::{
        AccountIdentifier, BlockIdentifier, Error, OperationIdentifier, OperationStatus,
        OperationStatusType, OperationType, TransactionIdentifier,
    },
    ApiError, CoinCache,
};
use anyhow::anyhow;
use aptos_crypto::{ed25519::Ed25519PublicKey, ValidCryptoMaterialStringExt};
use aptos_logger::warn;
use aptos_rest_client::aptos_api_types::TransactionOnChainData;
use aptos_rest_client::aptos_api_types::U64;
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_types::account_config::{AccountResource, CoinStoreResource, WithdrawEvent};
use aptos_types::contract_event::ContractEvent;
use aptos_types::stake_pool::StakePool;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use aptos_types::write_set::WriteOp;
use aptos_types::{account_address::AccountAddress, event::EventKey};
use cached_packages::aptos_stdlib;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Arc;
use std::{
    collections::HashMap,
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
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BalanceExemption.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BalanceExemption {}

/// Representation of a Block for a blockchain.  For aptos it is the version
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
    ///
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
        address: AccountAddress,
        amount: Option<Amount>,
        metadata: Option<OperationMetadata>,
    ) -> Operation {
        Operation {
            operation_identifier: OperationIdentifier {
                index: operation_index,
            },
            operation_type: operation_type.to_string(),
            status: status.map(|inner| inner.to_string()),
            account: Some(address.into()),
            amount,
            metadata,
        }
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
            address,
            None,
            Some(OperationMetadata::create_account(sender)),
        )
    }

    pub fn deposit(
        operation_index: u64,
        status: Option<OperationStatusType>,
        address: AccountAddress,
        currency: Currency,
        amount: u64,
    ) -> Operation {
        Operation::new(
            OperationType::Deposit,
            operation_index,
            status,
            address,
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
        address: AccountAddress,
        currency: Currency,
        amount: u64,
    ) -> Operation {
        Operation::new(
            OperationType::Withdraw,
            operation_index,
            status,
            address,
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
            address,
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
        address: AccountAddress,
        operator: AccountAddress,
    ) -> Operation {
        Operation::new(
            OperationType::SetOperator,
            operation_index,
            status,
            address,
            None,
            Some(OperationMetadata::set_operator(operator)),
        )
    }

    pub fn set_voter(
        operation_index: u64,
        status: Option<OperationStatusType>,
        address: AccountAddress,
        voter: AccountAddress,
    ) -> Operation {
        Operation::new(
            OperationType::SetVoter,
            operation_index,
            status,
            address,
            None,
            Some(OperationMetadata::set_voter(voter)),
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
            }
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
    pub voter: Option<AccountIdentifier>,
}

impl OperationMetadata {
    pub fn create_account(sender: AccountAddress) -> Self {
        OperationMetadata {
            sender: Some(sender.into()),
            ..Default::default()
        }
    }

    pub fn set_operator(operator: AccountAddress) -> Self {
        OperationMetadata {
            operator: Some(operator.into()),
            ..Default::default()
        }
    }

    pub fn set_voter(voter: AccountAddress) -> Self {
        OperationMetadata {
            voter: Some(voter.into()),
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
    StateCheckpoint,
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use TransactionType::*;
        f.write_str(match self {
            User => "User",
            Genesis => "Genesis",
            BlockMetadata => "BlockResource",
            StateCheckpoint => "StateCheckpoint",
        })
    }
}

impl Transaction {
    pub async fn from_transaction(
        coin_cache: Arc<CoinCache>,
        txn: TransactionOnChainData,
    ) -> ApiResult<Transaction> {
        use aptos_types::transaction::Transaction::*;
        let (txn_type, maybe_user_txn, txn_info, events) = match &txn.transaction {
            UserTransaction(user_txn) => {
                (TransactionType::User, Some(user_txn), txn.info, txn.events)
            }
            GenesisTransaction(_) => (TransactionType::Genesis, None, txn.info, txn.events),
            BlockMetadata(_) => (TransactionType::BlockMetadata, None, txn.info, txn.events),
            StateCheckpoint(_) => (TransactionType::StateCheckpoint, None, txn.info, vec![]),
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
                    coin_cache.clone(),
                    state_key,
                    write_op,
                    &events,
                    maybe_user_txn.map(|inner| inner.sender()),
                    maybe_user_txn.map(|inner| inner.payload()),
                    txn.version,
                    operation_index,
                )
                .await?;
                operation_index += ops.len() as u64;
                operations.append(&mut ops);
            }
        } else {
            // Parse all failed operations from the payload
            if let Some(user_txn) = maybe_user_txn {
                let mut ops = parse_failed_operations_from_txn_payload(
                    coin_cache.clone(),
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
    coin_cache: Arc<CoinCache>,
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
                    if let Some(currency) = coin_cache.get_currency_from_cache(type_tag) {
                        operations = parse_transfer_from_txn_payload(
                            inner,
                            currency,
                            sender,
                            operation_index,
                        )
                    }
                }
            }
            (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNCTION) => {
                // We could add a create here as well, but we don't know if it will actually happen
                operations =
                    parse_transfer_from_txn_payload(inner, native_coin(), sender, operation_index)
            }
            (AccountAddress::ONE, ACCOUNT_MODULE, CREATE_ACCOUNT_FUNCTION) => {
                if let Some(Ok(address)) = inner
                    .args()
                    .get(0)
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
            }
            (AccountAddress::ONE, STAKE_MODULE, SET_OPERATOR_FUNCTION) => {
                if let Some(Ok(operator)) = inner
                    .args()
                    .get(0)
                    .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded))
                {
                    operations.push(Operation::set_operator(
                        operation_index,
                        Some(OperationStatusType::Failure),
                        sender,
                        operator,
                    ));
                } else {
                    warn!("Failed to parse set operator {:?}", inner);
                }
            }
            (AccountAddress::ONE, STAKE_MODULE, SET_VOTER_FUNCTION) => {
                if let Some(Ok(voter)) = inner
                    .args()
                    .get(0)
                    .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded))
                {
                    operations.push(Operation::set_voter(
                        operation_index,
                        Some(OperationStatusType::Failure),
                        sender,
                        voter,
                    ));
                } else {
                    warn!("Failed to parse set voter {:?}", inner);
                }
            }
            _ => {
                // If we don't recognize the transaction payload, then we can't parse operations
            }
        }
    }
    operations
}

fn parse_transfer_from_txn_payload(
    payload: &EntryFunction,
    currency: Currency,
    sender: AccountAddress,
    operation_index: u64,
) -> Vec<Operation> {
    let mut operations = vec![];

    let args = payload.args();
    let maybe_receiver = args
        .get(0)
        .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded));
    let maybe_amount = args.get(1).map(|encoded| bcs::from_bytes::<u64>(encoded));

    if let (Some(Ok(receiver)), Some(Ok(amount))) = (maybe_receiver, maybe_amount) {
        operations.push(Operation::withdraw(
            operation_index,
            Some(OperationStatusType::Failure),
            sender,
            currency.clone(),
            amount,
        ));
        operations.push(Operation::deposit(
            operation_index + 1,
            Some(OperationStatusType::Failure),
            receiver,
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
    coin_cache: Arc<CoinCache>,
    state_key: &StateKey,
    write_op: &WriteOp,
    events: &[ContractEvent],
    maybe_sender: Option<AccountAddress>,
    maybe_payload: Option<&TransactionPayload>,
    version: u64,
    operation_index: u64,
) -> ApiResult<Vec<Operation>> {
    let operations = vec![];

    // If we have any entry functions that don't provide proper event based changes, we have to
    // pull the changes possibly from the payload.  This is more fragile, as it doesn't support
    // move scripts and wrapper entry functions.
    if let (Some(TransactionPayload::EntryFunction(inner)), Some(sender)) =
        (maybe_payload, maybe_sender)
    {
        if let (AccountAddress::ONE, STAKE_MODULE, SET_VOTER_FUNCTION) = (
            *inner.module().address(),
            inner.module().name().as_str(),
            inner.function().as_str(),
        ) {
            return if let Some(Ok(voter)) = inner
                .args()
                .get(0)
                .map(|encoded| bcs::from_bytes::<AccountAddress>(encoded))
            {
                Ok(vec![Operation::set_voter(
                    operation_index,
                    Some(OperationStatusType::Success),
                    sender,
                    voter,
                )])
            } else {
                warn!("Failed to parse set voter {:?}", inner);
                Ok(vec![])
            };
        }
    }

    let (struct_tag, address) = match state_key {
        StateKey::AccessPath(path) => {
            if let Some(struct_tag) = path.get_struct_tag() {
                (struct_tag, path.address)
            } else {
                return Ok(vec![]);
            }
        }
        _ => {
            // Ignore all but access path
            return Ok(vec![]);
        }
    };

    let data = match write_op {
        WriteOp::Creation(inner) => inner,
        WriteOp::Modification(inner) => inner,
        WriteOp::Deletion => return Ok(vec![]),
    };

    // Determine operation
    match (
        struct_tag.address,
        struct_tag.module.as_str(),
        struct_tag.name.as_str(),
        struct_tag.type_params.len(),
    ) {
        (AccountAddress::ONE, ACCOUNT_MODULE, ACCOUNT_RESOURCE, 0) => {
            parse_account_changes(version, address, data, maybe_sender, operation_index)
        }
        (AccountAddress::ONE, STAKE_MODULE, STAKE_POOL_RESOURCE, 0) => {
            parse_stakepool_changes(version, address, data, events, operation_index)
        }
        (AccountAddress::ONE, COIN_MODULE, COIN_STORE_RESOURCE, 1) => {
            if let Some(type_tag) = struct_tag.type_params.first() {
                parse_coinstore_changes(
                    coin_cache,
                    type_tag.clone(),
                    version,
                    address,
                    data,
                    events,
                    operation_index,
                )
                .await
            } else {
                warn!(
                    "Failed to parse coinstore {} at version {}",
                    struct_tag, version
                );
                Ok(operations)
            }
        }
        _ => {
            // Any unknown type will just skip the operations
            Ok(operations)
        }
    }
}

fn parse_account_changes(
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

fn parse_stakepool_changes(
    version: u64,
    address: AccountAddress,
    data: &[u8],
    events: &[ContractEvent],
    mut operation_index: u64,
) -> ApiResult<Vec<Operation>> {
    let mut operations = Vec::new();
    if let Ok(stakepool) = bcs::from_bytes::<StakePool>(data) {
        stakepool.set_operator_events.key();

        let addresses = get_set_operator_from_event(events, stakepool.set_operator_events.key());
        for operator in addresses {
            operations.push(Operation::set_operator(
                operation_index,
                Some(OperationStatusType::Success),
                address,
                operator,
            ));
            operation_index += 1;
        }
    } else {
        warn!(
            "Failed to parse stakepool for {} at version {}",
            address, version
        );
    }

    Ok(operations)
}

async fn parse_coinstore_changes(
    coin_cache: Arc<CoinCache>,
    coin_type: TypeTag,
    version: u64,
    address: AccountAddress,
    data: &[u8],
    events: &[ContractEvent],
    mut operation_index: u64,
) -> ApiResult<Vec<Operation>> {
    let coin_store: CoinStoreResource = if let Ok(coin_store) = bcs::from_bytes(data) {
        coin_store
    } else {
        warn!(
            "Coin store failed to parse for coin type {} and address {} at version {}",
            coin_type, address, version
        );
        return Ok(vec![]);
    };

    let mut operations = vec![];

    // Retrieve the coin type
    let currency = coin_cache
        .get_currency(coin_type.clone(), Some(version))
        .await
        .map_err(|err| {
            ApiError::CoinTypeFailedToBeFetched(Some(format!(
                "Failed to retrieve coin type {} for {} at version {}: {}",
                coin_type, address, version, err
            )))
        })?;

    // Skip if there is no currency that can be found
    if let Some(currency) = currency.as_ref() {
        let withdraw_amounts = get_amount_from_event(events, coin_store.withdraw_events().key());
        for amount in withdraw_amounts {
            operations.push(Operation::withdraw(
                operation_index,
                Some(OperationStatusType::Success),
                address,
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
                address,
                currency.clone(),
                amount,
            ));
            operation_index += 1;
        }

        if operations.is_empty() {
            warn!(
                "No transfer operations found for {} coinstore for {} at version {}",
                currency
                    .metadata
                    .as_ref()
                    .map(|inner| inner.move_type.as_str())
                    .unwrap_or("Unknown move type"),
                address,
                version
            );
        }
    } else {
        warn!(
            "Currency {} is invalid for {} at version {}",
            coin_type, address, version
        );
    }

    Ok(operations)
}

/// Pulls the balance change from a withdraw or deposit event
fn get_amount_from_event(events: &[ContractEvent], event_key: &EventKey) -> Vec<u64> {
    filter_events(events, event_key, |event| {
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

fn get_set_operator_from_event(
    events: &[ContractEvent],
    event_key: &EventKey,
) -> Vec<AccountAddress> {
    filter_events(events, event_key, |event| {
        if let Ok(event) = bcs::from_bytes::<SetOperatorEvent>(event.event_data()) {
            Some(event.new_operator)
        } else {
            warn!(
                "Failed to parse set operator event!  Skipping for {}:{}",
                event_key.get_creator_address(),
                event_key.get_creation_number()
            );
            None
        }
    })
}

fn filter_events<F: FnMut(&ContractEvent) -> Option<T>, T>(
    events: &[ContractEvent],
    event_key: &EventKey,
    parser: F,
) -> Vec<T> {
    events
        .iter()
        .filter(|event| event.key() == event_key)
        .sorted_by(|a, b| a.sequence_number().cmp(&b.sequence_number()))
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

/// A holder for all information related to a specific transaction
/// built from [`Operation`]s
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InternalOperation {
    CreateAccount(CreateAccount),
    Transfer(Transfer),
    SetOperator(SetOperator),
    SetVoter(SetVoter),
}

impl InternalOperation {
    /// Pulls the [`InternalOperation`] from the set of [`Operation`]
    pub fn extract(operations: &Vec<Operation>) -> ApiResult<InternalOperation> {
        match operations.len() {
            1 => {
                if let Some(operation) = operations.first() {
                    match OperationType::from_str(&operation.operation_type) {
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
                        }
                        Ok(OperationType::SetOperator) => {
                            if let (
                                Some(OperationMetadata {
                                    operator: Some(operator),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::SetOperator(SetOperator {
                                    owner: account.account_address()?,
                                    operator: operator.account_address()?,
                                }));
                            }
                        }
                        Ok(OperationType::SetVoter) => {
                            if let (
                                Some(OperationMetadata {
                                    voter: Some(voter), ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::SetVoter(SetVoter {
                                    owner: account.account_address()?,
                                    voter: voter.account_address()?,
                                }));
                            }
                        }
                        _ => {}
                    }
                }

                // Return invalid operations if for any reason parsing fails
                Err(ApiError::InvalidOperations(Some(format!(
                    "Unrecognized single operation {:?}",
                    operations
                ))))
            }
            2 => Ok(Self::Transfer(Transfer::extract_transfer(operations)?)),
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
            }
            InternalOperation::SetOperator(set_operator) => (
                aptos_stdlib::stake_set_operator(set_operator.operator),
                set_operator.owner,
            ),
            InternalOperation::SetVoter(set_voter) => (
                aptos_stdlib::stake_set_delegated_voter(set_voter.voter),
                set_voter.owner,
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
        if !op_map.contains_key(&OperationType::Withdraw) {}

        if !op_map.contains_key(&OperationType::Deposit) {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have a deposit",
            )));
        }

        // Verify accounts and amounts
        let (sender, withdraw_amount) = if let Some(withdraw) = op_map.get(&OperationType::Withdraw)
        {
            if let (Some(account), Some(amount)) = (&withdraw.account, &withdraw.amount) {
                (AccountAddress::try_from(account)?, amount)
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
                (AccountAddress::try_from(account)?, amount)
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
    pub operator: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetVoter {
    pub owner: AccountAddress,
    pub voter: AccountAddress,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SetOperatorEvent {
    pub pool_address: AccountAddress,
    pub old_operator: AccountAddress,
    pub new_operator: AccountAddress,
}
