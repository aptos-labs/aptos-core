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
