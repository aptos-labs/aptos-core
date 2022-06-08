// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    AccountIdentifier, BlockIdentifier, Error, NetworkIdentifier, OperationIdentifier,
    OperationStatus, TransactionIdentifier,
};
use serde::{Deserialize, Serialize};

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
    /// A list of balance exemptions, where these accounts change their balance
    /// without an operation
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

///
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BalanceExemption.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BalanceExemption {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exemption_type: Option<ExemptionType>,
}

///
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Block.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Block {
    pub block_identifier: BlockIdentifier,
    pub parent_block_identifier: BlockIdentifier,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
}

///
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockEvent.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockEvent {
    pub sequence: u64,
    pub block_identifier: BlockIdentifier,
    #[serde(rename = "type")]
    pub block_event_type: BlockEventType,
    pub transactions: Vec<Transaction>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BlockEventType.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockEventType {
    BlockAdded,
    BlockRemoved,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BlockTransaction.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockTransaction {
    block_identifier: BlockIdentifier,
    transaction: Transaction,
}

/// Tells
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Currency {
    /// Symbol of currency
    pub symbol: String,
    /// Number of decimals to be considered in the currecny
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

/// [API Spec](https://www.rosetta-api.org/docs/models/Direction.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Backward,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ExemptionType.html)
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExemptionType {
    GreaterOrEqual,
    LessOrEqual,
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
}

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

/// [API Spec](https://www.rosetta-api.org/docs/models/PublicKey.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PublicKey {
    /// Hex encoded public key bytes
    pub hex_bytes: String,
    pub curve_type: CurveType,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/RelatedTransaction.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelatedTransaction {
    pub network_identifier: Option<NetworkIdentifier>,
    pub transaction_identifier: TransactionIdentifier,
    pub direction: Direction,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/Signature.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Signature {
    pub signing_payload: SigningPayload,
    pub public_key: PublicKey,
    pub signature_type: SignatureType,
    /// Hex bytes of
    pub hex_bytes: String,
}

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

/// [API Spec](https://www.rosetta-api.org/docs/models/SigningPayload.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SigningPayload {
    /// Deprecated field, replaced with account_identifier
    pub address: Option<String>,
    pub account_identifier: Option<AccountIdentifier>,
    /// Hex encoded string of payload bytes
    pub hex_bytes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_type: Option<SignatureType>,
}

///
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Transaction.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transaction {
    pub transaction_identifier: TransactionIdentifier,
    pub operations: Vec<Operation>,
    pub related_transactions: Option<Vec<RelatedTransaction>>,
}
