// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{is_native_coin, native_coin},
    error::ApiResult,
    types::{
        AccountIdentifier, BlockIdentifier, Error, NetworkIdentifier, OperationIdentifier,
        OperationStatus, OperationStatusType, OperationType, TransactionIdentifier,
    },
    ApiError, CoinCache,
};
use anyhow::anyhow;
use aptos_crypto::{ed25519::Ed25519PublicKey, ValidCryptoMaterialStringExt};
use aptos_logger::info;
use aptos_rest_client::{
    aptos::Balance,
    aptos_api_types::{WriteSetChange, U64},
};
use aptos_sdk::move_types::{ident_str, identifier::Identifier};
use aptos_types::{account_address::AccountAddress, event::EventKey};
use itertools::Itertools;
use move_deps::move_core_types::language_storage::{StructTag, TypeTag};
use serde::{de::Error as SerdeError, Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    str::FromStr,
    sync::Arc,
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
    /// TODO: Determine if we even need to bother with this
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start_index: Option<u64>,
    /// All call methods supported
    pub call_methods: Vec<String>,
    /// A list of balance exemptions.  These should be as minimal as possible, otherwise it becomes
    /// more complicated for users
    pub balance_exemptions: Vec<BalanceExemption>,
    /// Determines if mempool can change the balance on an account
    /// This should be set to false
    pub mempool_coins: bool,
    /// Case specifics for block hashes.  Set to None if case insensitive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash_case: Option<Case>,
    /// Case specifics for transaction hashes.  Set to None if case insensitive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_hash_case: Option<Case>,
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

impl From<Balance> for Amount {
    fn from(balance: Balance) -> Self {
        Amount {
            value: balance.coin.value.to_string(),
            // TODO: Support other currencies
            currency: native_coin(),
        }
    }
}

/// Balance exemptions where the current balance of an account can change without a transaction
/// operation.  This is typically e
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BalanceExemption.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BalanceExemption {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account_address: Option<String>,
    /// The currency that can change based on the exemption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    /// The exemption type of which direction a balance can change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exemption_type: Option<ExemptionType>,
}

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

/// Events that allow lighter weight block updates of add and removing block
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockEvent.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockEvent {
    /// Ordered event index for events on a NetworkIdentifier (likely the same as version)
    pub sequence: u64,
    /// Block identifier of the block to change
    pub block_identifier: BlockIdentifier,
    /// Block event type add or remove block
    #[serde(rename = "type")]
    pub block_event_type: BlockEventType,
    /// Transactions associated with the update, it should be only one transaction in Aptos.
    pub transactions: Vec<Transaction>,
}

/// Determines if the event is about adding or removing blocks
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockEventType.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockEventType {
    BlockAdded,
    BlockRemoved,
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

/// Tells what cases are supported in hashes. Having no value is case insensitive.
///
///[API Spec](https://www.rosetta-api.org/docs/models/Case.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    UpperCase,
    LowerCase,
    CaseSensitive,
}

/// Currency represented as atomic units including decimals
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Currency.html)
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Currency {
    /// Symbol of currency
    pub symbol: String,
    /// Number of decimals to be considered in the currency
    pub decimals: u64,
}

/// Various signing curves supported by Rosetta.  We only use [`CurveType::Edwards25519`]
/// [API Spec](https://www.rosetta-api.org/docs/models/CurveType.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveType {
    Edwards25519,
    Secp256k1,
    Secp256r1,
    Tweedle,
    Pallas,
}

/// Used for related transactions to determine direction of relation
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Direction.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Associated to a later transaction
    Forward,
    /// Associated to an earlier transaction
    Backward,
}

/// Tells how balances can change without a specific transaction on the account
/// TODO: Determine if we need to set these for staking
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ExemptionType.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExemptionType {
    /// Balance can be greater than or equal to the current balance e.g. staking
    GreaterOrEqual,
    /// Balance can be less than or equal to the current balance
    LessOrEqual,
    /// Balance can be less than or greater than the current balance e.g. dynamic supplies
    Dynamic,
}

/// A representation of a single account change in a transaction
///
/// This is known as a write set change within Aptos
/// [API Spec](https://www.rosetta-api.org/docs/models/Operation.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Operation {
    /// Identifier of an operation within a transaction
    pub operation_identifier: OperationIdentifier,
    /// Related operations e.g. multiple operations that are related to a transfer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_operations: Option<Vec<OperationIdentifier>>,
    /// Type of operation
    #[serde(rename = "type")]
    pub operation_type: String,
    /// Status of operation.  Must be populated if the transaction is in the past.  If submitting
    /// new transactions, it must NOT be populated.
    pub status: Option<String>,
    /// AccountIdentifier should be provided to point at which account the change is
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<AccountIdentifier>,
    /// Amount in the operation
    ///
    /// TODO: Determine if this is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    /// Operation specific metadata for any operation that's missing information it needs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<OperationSpecificMetadata>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationSpecificMetadata {
    /// Sender for operations that affect accounts other than the sender
    pub sender: AccountIdentifier,
}

/// Used for query operations to apply conditions.  Defaults to [`Operator::And`] if no value is
/// present
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Operator.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    And,
    Or,
}

impl Default for Operator {
    fn default() -> Self {
        Operator::And
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

/// Related Transaction allows for connecting related transactions across shards, networks or
/// other boundaries.
///
/// [API Spec](https://www.rosetta-api.org/docs/models/RelatedTransaction.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelatedTransaction {
    /// Network of transaction.  [`None`] means same network as original transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_identifier: Option<NetworkIdentifier>,
    /// Transaction identifier of the related transaction
    pub transaction_identifier: TransactionIdentifier,
    /// Direction of the relation (forward or backward in time)
    pub direction: Direction,
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
    Ecdsa,
    EcdsaRecovery,
    Ed25519,
    #[serde(rename = "schnoor_1")]
    Schnoor1,
    SchnoorPoseidon,
}

/// Signing payload should be signed by the client with their own private key
///
/// [API Spec](https://www.rosetta-api.org/docs/models/SigningPayload.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SigningPayload {
    /// Deprecated field, replaced with account_identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Account identifier of the signer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier: Option<AccountIdentifier>,
    /// Hex encoded string of payload bytes to be signed
    pub hex_bytes: String,
    /// Signature type to sign with
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Related transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_transactions: Option<Vec<RelatedTransaction>>,
}

impl Transaction {
    pub async fn from_transaction(
        coin_cache: Arc<CoinCache>,
        rest_client: &aptos_rest_client::Client,
        txn: aptos_rest_client::Transaction,
    ) -> ApiResult<Transaction> {
        use aptos_rest_client::Transaction::*;
        let (maybe_sender, txn_info, events) = match txn {
            // Pending transactions aren't supported by Rosetta (for now)
            PendingTransaction(_) => return Err(ApiError::TransactionIsPending),
            UserTransaction(txn) => (Some(txn.request.sender), txn.info, txn.events),
            GenesisTransaction(txn) => (None, txn.info, txn.events),
            BlockMetadataTransaction(txn) => (None, txn.info, txn.events),
            StateCheckpointTransaction(txn) => (None, txn.info, vec![]),
        };

        info!(
            "TRANSACTION: \n====\nTransaction: {}\n---\n Events: {}",
            serde_json::to_string_pretty(&txn_info).unwrap(),
            serde_json::to_string_pretty(&events).unwrap()
        );

        let mut operations = vec![];
        let mut operation_index: u64 = 0;
        let status = if txn_info.success {
            OperationStatusType::Success
        } else {
            OperationStatusType::Failure
        };

        // TODO: put these somewhere better
        let account: Identifier = ident_str!("Account").into();
        let coin_info: Identifier = ident_str!("CoinInfo").into();
        let coin_id: Identifier = ident_str!("Coin").into();
        let coin_store: Identifier = ident_str!("CoinStore").into();
        let sequence_number: Identifier = ident_str!("sequence_number").into();
        let deposit_events: Identifier = ident_str!("deposit_events").into();
        let withdraw_events: Identifier = ident_str!("withdraw_events").into();
        let decimals_id: Identifier = ident_str!("decimals").into();
        let symbol_id: Identifier = ident_str!("symbol").into();

        for change in &txn_info.changes {
            // TODO: Handle delete resource ?
            if let WriteSetChange::WriteResource { address, data, .. } = change {
                // Determine operation
                let address = *address.inner();
                let module = data.typ.module.clone();
                let name = data.typ.name.clone();
                let generic_type_params = &data.typ.generic_type_params;

                // Only handle framework events for now
                let op_details = if *data.typ.address.inner() == AccountAddress::ONE {
                    let mut op_details = None;
                    if module == account && name == account {
                        // Account sequence number increase (possibly creation)
                        // Find out if it's the 0th sequence number (creation)
                        for (id, value) in data.data.0.iter() {
                            if id == &sequence_number {
                                if let Ok(U64(0)) = serde_json::from_value::<U64>(value.clone()) {
                                    op_details = Some(OperationDetails::CreateAccount);
                                    break;
                                }
                            }
                        }
                    } else if module == coin_id && name == coin_info {
                        // Coin creation
                        let mut decimals: Option<u64> = None;
                        let mut symbol = None;

                        // Find the coin details
                        for (id, value) in data.data.0.iter() {
                            if id == &decimals_id {
                                if let Ok(U64(dec)) = serde_json::from_value::<U64>(value.clone()) {
                                    decimals = Some(dec);
                                }
                            } else if id == &symbol_id {
                                if let Ok(sym) = serde_json::from_value::<String>(value.clone()) {
                                    symbol = Some(sym);
                                }
                            }
                        }

                        // Only if we got all the fields do we use it
                        if let (Some(decimals), Some(symbol), Some(coin_type)) =
                            (decimals, symbol, generic_type_params.first())
                        {
                            op_details = Some(OperationDetails::CreateCoin {
                                coin_type: coin_type.to_string(),
                                symbol,
                                decimals,
                            })
                        }
                    } else if module == coin_id && name == coin_store {
                        if let Some(coin) = generic_type_params.first() {
                            // Account balance change
                            let mut withdraw_event = None;
                            let mut deposit_event = None;

                            // Find the coin details
                            for (id, value) in data.data.0.iter() {
                                if id == &withdraw_events {
                                    serde_json::from_value::<CoinEventId>(value.clone()).unwrap();
                                    if let Ok(event) =
                                        serde_json::from_value::<CoinEventId>(value.clone())
                                    {
                                        withdraw_event = Some(EventKey::new_from_address(
                                            &event.guid.guid.id.addr,
                                            event.guid.guid.id.creation_num.0,
                                        ));
                                    }
                                } else if id == &deposit_events {
                                    if let Ok(event) =
                                        serde_json::from_value::<CoinEventId>(value.clone())
                                    {
                                        deposit_event = Some(EventKey::new_from_address(
                                            &address,
                                            event.guid.guid.id.creation_num.0,
                                        ));
                                    }
                                }
                            }

                            // Some transfers are onesided (e.g. mints)
                            if withdraw_event.is_some() || deposit_event.is_some() {
                                if let Ok(coin_type) = TypeTag::try_from(coin.clone()) {
                                    if let Some(currency) = coin_cache
                                        .get_currency(
                                            rest_client,
                                            coin_type.clone(),
                                            Some(txn_info.version.0),
                                        )
                                        .await?
                                    {
                                        op_details = Some(OperationDetails::TransferCoin {
                                            currency,
                                            withdraw_event_key: withdraw_event,
                                            deposit_event_key: deposit_event,
                                        });
                                    } else {
                                        return Err(ApiError::UnsupportedCurrency(format!(
                                            "Currency {} is not supported",
                                            coin_type
                                        )));
                                    }
                                }
                            }
                        }
                    }
                    op_details
                } else {
                    None
                };

                // TODO: support coin creation?
                match op_details {
                    Some(OperationDetails::CreateAccount) => {
                        operations.push(Operation {
                            operation_identifier: OperationIdentifier {
                                index: operation_index,
                                network_index: None,
                            },
                            related_operations: None,
                            operation_type: OperationType::CreateAccount.to_string(),
                            status: Some(status.to_string()),
                            account: Some(AccountIdentifier::from(address)),
                            amount: None,
                            metadata: Some(OperationSpecificMetadata {
                                sender: maybe_sender
                                    .map(|address| *address.inner())
                                    .unwrap_or(AccountAddress::ONE)
                                    .into(),
                            }),
                        });
                        operation_index += 1;
                    }
                    Some(OperationDetails::TransferCoin {
                        currency,
                        deposit_event_key,
                        withdraw_event_key,
                    }) => {
                        // Determine amount change this is silly, cause you have to pull it from the events
                        if let Some(event_key) = deposit_event_key {
                            if let Some(event) = events
                                .iter()
                                .find(|event| EventKey::from(event.key) == event_key)
                            {
                                if let Ok(CoinEvent { amount }) =
                                    serde_json::from_value::<CoinEvent>(event.data.clone())
                                {
                                    operations.push(Operation {
                                        operation_identifier: OperationIdentifier {
                                            index: operation_index,
                                            network_index: None,
                                        },
                                        related_operations: None,
                                        operation_type: OperationType::Deposit.to_string(),
                                        status: Some(status.to_string()),
                                        account: Some(AccountIdentifier::from(address)),
                                        amount: Some(Amount {
                                            value: amount.to_string(),
                                            currency: currency.clone(),
                                        }),
                                        metadata: None,
                                    });
                                    operation_index += 1;
                                }
                            }
                        }

                        if let Some(event_key) = withdraw_event_key {
                            if let Some(event) = events
                                .iter()
                                .find(|event| EventKey::from(event.key) == event_key)
                            {
                                if let Ok(CoinEvent { amount }) =
                                    serde_json::from_value::<CoinEvent>(event.data.clone())
                                {
                                    operations.push(Operation {
                                        operation_identifier: OperationIdentifier {
                                            index: operation_index,
                                            network_index: None,
                                        },
                                        related_operations: None,
                                        operation_type: OperationType::Withdraw.to_string(),
                                        status: Some(status.to_string()),
                                        account: Some(AccountIdentifier::from(address)),
                                        amount: Some(Amount {
                                            value: format!("-{}", amount),
                                            currency: currency.clone(),
                                        }),
                                        metadata: None,
                                    });
                                    operation_index += 1;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Also add a gas removal
        if let Some(sender) = maybe_sender {
            let gas_tag = TypeTag::Struct(StructTag {
                address: AccountAddress::ONE,
                module: ident_str!("TestCoin").into(),
                name: ident_str!("TestCoin").into(),
                type_params: vec![],
            });
            if let Some(gas_currency) = coin_cache.get_currency(rest_client, gas_tag, None).await? {
                operations.push(Operation {
                    operation_identifier: OperationIdentifier {
                        index: operation_index,
                        network_index: None,
                    },
                    related_operations: None,
                    operation_type: OperationType::Withdraw.to_string(),
                    status: Some(status.to_string()),
                    account: Some(AccountIdentifier::from(*sender.inner())),
                    amount: Some(Amount {
                        value: format!("-{}", txn_info.gas_used),
                        currency: gas_currency,
                    }),
                    metadata: None,
                });
            } else {
                return Err(ApiError::AptosError(
                    "Failed to load gas currency".to_string(),
                ));
            }
        }

        Ok(Transaction {
            transaction_identifier: (&txn_info).into(),
            operations,
            related_transactions: None,
        })
    }
}

pub enum OperationDetails {
    CreateAccount,
    CreateCoin {
        coin_type: String,
        symbol: String,
        decimals: u64,
    },
    TransferCoin {
        currency: Currency,
        withdraw_event_key: Option<EventKey>,
        deposit_event_key: Option<EventKey>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InternalOperation {
    CreateAccount(CreateAccount),
    Transfer(Transfer),
}

impl InternalOperation {
    pub fn extract(operations: &Vec<Operation>) -> ApiResult<InternalOperation> {
        match operations.len() {
            1 => {
                if let Some(operation) = operations.first() {
                    if operation.operation_type == OperationType::CreateAccount.to_string() {
                        if let (Some(OperationSpecificMetadata { sender }), Some(account)) =
                            (&operation.metadata, &operation.account)
                        {
                            return Ok(Self::CreateAccount(CreateAccount {
                                sender: sender.account_address()?,
                                new_account: account.account_address()?,
                            }));
                        }
                    }
                }

                Err(ApiError::InvalidOperations)
            }
            2 => Ok(Self::Transfer(Transfer::extract_transfer(operations)?)),
            _ => Err(ApiError::InvalidOperations),
        }
    }

    pub fn sender(&self) -> AccountAddress {
        match self {
            Self::CreateAccount(inner) => inner.sender,
            Self::Transfer(inner) => inner.sender,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateAccount {
    pub sender: AccountAddress,
    pub new_account: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transfer {
    pub sender: AccountAddress,
    pub receiver: AccountAddress,
    pub amount: u64,
    pub currency: Currency,
}

impl Transfer {
    pub fn extract_transfer(operations: &Vec<Operation>) -> ApiResult<Transfer> {
        // Only support 1:1 P2P transfer
        // This is composed of a Deposit and a Withdraw operation
        if operations.len() != 2
            && (!operations.iter().any(|op| {
                OperationType::from_str(&op.operation_type).unwrap_or(OperationType::Withdraw)
                    == OperationType::Deposit
            }) || !operations.iter().any(|op| {
                OperationType::from_str(&op.operation_type).unwrap_or(OperationType::Deposit)
                    == OperationType::Withdraw
            }))
        {
            return Err(ApiError::InvalidTransferOperations(
                "Must have exactly 1 withdraw and 1 deposit".to_string(),
            ));
        }

        let mut op_map = HashMap::new();
        for op in operations {
            let op_type = OperationType::from_str(&op.operation_type)?;
            op_map.insert(op_type, op);
        }
        let mut keys = op_map.keys();
        if !keys.contains(&OperationType::Withdraw) || !keys.contains(&OperationType::Deposit) {
            return Err(ApiError::InvalidTransferOperations(
                "Must have exactly 1 withdraw and 1 deposit".to_string(),
            ));
        }

        // Verify accounts and amounts
        let withdraw = op_map.get(&OperationType::Withdraw).unwrap();
        let sender = if let Some(ref account) = withdraw.account {
            account.try_into()?
        } else {
            return Err(ApiError::InvalidTransferOperations(
                "Invalid withdraw account provided".to_string(),
            ));
        };

        let deposit = op_map.get(&OperationType::Deposit).unwrap();
        let receiver = if let Some(ref account) = deposit.account {
            account.try_into()?
        } else {
            return Err(ApiError::InvalidTransferOperations(
                "Invalid deposit account provided".to_string(),
            ));
        };

        let (amount, currency): (u64, Currency) =
            if let (Some(withdraw_amount), Some(deposit_amount)) =
                (&withdraw.amount, &deposit.amount)
            {
                // Currencies have to be the same
                if withdraw_amount.currency != deposit_amount.currency {
                    return Err(ApiError::InvalidTransferOperations(
                        "Currency mismatch between withdraw and deposit".to_string(),
                    ));
                }

                // Check that the currency is supported
                // TODO: in future use currency, since there's more than just 1
                let _ = is_native_coin(&withdraw_amount.currency)?;

                let withdraw_value = i64::from_str(&withdraw_amount.value).map_err(|_| {
                    ApiError::InvalidTransferOperations("Withdraw amount is invalid".to_string())
                })?;
                let deposit_value = i64::from_str(&deposit_amount.value).map_err(|_| {
                    ApiError::InvalidTransferOperations("Deposit amount is invalid".to_string())
                })?;

                // We can't create or destroy coins, they must be negatives of each other
                if -withdraw_value != deposit_value {
                    return Err(ApiError::InvalidTransferOperations(
                        "Withdraw amount must be equal to negative of deposit amount".to_string(),
                    ));
                }

                (deposit_value as u64, deposit_amount.currency.clone())
            } else {
                return Err(ApiError::InvalidTransferOperations(
                    "Must have exactly 1 withdraw and 1 deposit with amounts".to_string(),
                ));
            };

        Ok(Transfer {
            sender,
            receiver,
            amount,
            currency,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CoinEvent {
    amount: U64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CoinEventId {
    guid: Guid,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Guid {
    guid: Id,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Id {
    id: EventId,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EventId {
    #[serde(deserialize_with = "deserialize_account_address")]
    addr: AccountAddress,
    creation_num: U64,
}

fn deserialize_account_address<'de, D>(
    deserializer: D,
) -> std::result::Result<AccountAddress, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        let s = <String>::deserialize(deserializer)?;
        AccountAddress::from_hex_literal(&s).map_err(D::Error::custom)
    } else {
        // In order to preserve the Serde data model and help analysis tools,
        // make sure to wrap our value in a container with the same name
        // as the original type.
        #[derive(::serde::Deserialize)]
        #[serde(rename = "AccountAddress")]
        struct Value([u8; AccountAddress::LENGTH]);

        let value = Value::deserialize(deserializer)?;
        Ok(AccountAddress::new(value.0))
    }
}
