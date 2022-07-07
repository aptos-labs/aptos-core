// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    AccountIdentifier, Allow, Amount, Block, BlockIdentifier, Currency, InternalOperation,
    NetworkIdentifier, Operation, PartialBlockIdentifier, Peer, PublicKey, Signature,
    SigningPayload, SyncStatus, Transaction, TransactionIdentifier, Version,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

/// Request for an account's currency balance either now, or historically
///
/// [API Spec](https://www.rosetta-api.org/docs/models/AccountBalanceRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountBalanceRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// Account identifier describing the account address
    pub account_identifier: AccountIdentifier,
    /// For historical balance lookups by either hash or version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_identifier: Option<PartialBlockIdentifier>,
    /// For filtering which currencies to show
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currencies: Option<Vec<Currency>>,
}

/// Response with the version associated and the balances of the account
///
/// [API Spec](https://www.rosetta-api.org/docs/models/AccountBalanceResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountBalanceResponse {
    /// Block containing the balance
    pub block_identifier: BlockIdentifier,
    /// Balances of all known currencies
    pub balances: Vec<Amount>,
}

/// Reqyest a block (version) on the account
///
/// With neither value for PartialBlockIdentifier, get the latest version
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// A set of search parameters (latest, by hash, or by index)
    pub block_identifier: PartialBlockIdentifier,
}

impl BlockRequest {
    fn new(chain_id: ChainId, block_identifier: PartialBlockIdentifier) -> Self {
        Self {
            network_identifier: chain_id.into(),
            block_identifier,
        }
    }

    pub fn latest(chain_id: ChainId) -> Self {
        Self::new(chain_id, PartialBlockIdentifier::latest())
    }

    pub fn by_hash(chain_id: ChainId, hash: String) -> Self {
        Self::new(chain_id, PartialBlockIdentifier::by_hash(hash))
    }

    pub fn by_version(chain_id: ChainId, version: u64) -> Self {
        Self::new(chain_id, PartialBlockIdentifier::by_version(version))
    }
}

/// Response that will always have a valid block populated
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockResponse {
    /// The block requested.  This should always be populated for a given valid version
    pub block: Option<Block>,
    /// Transactions that weren't included in the block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_transactions: Option<Vec<TransactionIdentifier>>,
}

/// Request to combine signatures and an unsigned transaction for submission as a
/// [`aptos_types::transaction::SignedTransaction`]
///
/// This should be able to run without a running full node connection
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionCombineRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionCombineRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// A hex encoded, BCS encoded, [`aptos_types::transaction::RawTransaction`]
    pub unsigned_transaction: String,
    /// Set of signatures with SigningPayloads to combine
    pub signatures: Vec<Signature>,
}

/// Response of signed transaction for submission
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionCombineResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionCombineResponse {
    /// A hex encoded, BCS encoded, [`aptos_types::transaction::SignedTransaction`]
    pub signed_transaction: String,
}

/// Request to derive an account from a public key
///
/// This should be able to run without a running full node connection, but note that
/// this will not work with accounts that have rotated their public key.  It should
/// only be used when an account is being created.
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionDeriveRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionDeriveRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// Public key to derive an [`aptos_types::account_address::AccountAddress`] from
    pub public_key: PublicKey,
}

/// Response of derived account from a public key
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionDeriveResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionDeriveResponse {
    /// The account identifier of the account if the [`aptos_types::account_address::AccountAddress`] can be derived.
    ///
    /// This will always return a value, though it might not match onchain information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier: Option<AccountIdentifier>,
}

/// Request to hash a transaction
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionHashRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionHashRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// A hex encoded, BCS encoded, [`aptos_types::transaction::SignedTransaction`]
    pub signed_transaction: String,
}

/// Request to retrieve all information needed for constructing a transaction from the blockchain
///
/// A running full node is required for this API
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionMetadataRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionMetadataRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// Information telling which metadata to lookup onchain
    ///
    /// This comes verbatim from a preprocess request
    pub options: MetadataOptions,
    /// Set of public keys related to the private keys signing the transaction eventually
    pub public_keys: Vec<PublicKey>,
}

/// A set of operations to tell us which metadata to lookup onchain
///
/// This is built from Preprocess, and is copied verbatim to the metadata request
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MetadataOptions {
    /// The operation to run at a high level (e.g. CreateAccount/Transfer)
    pub internal_operation: InternalOperation,
    /// Maximum gas willing to pay for the transaction
    pub max_gas: u64,
    /// Multiplier e.g. how much each unit of gas is worth in the native coin
    pub gas_price_per_unit: u64,
}

/// Response with network specific data for constructing a transaction
///
/// In this case, sequence number must be pulled from onchain.
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionMetadataResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionMetadataResponse {
    /// Metadata that will be passed to Payloads to create a transaction
    pub metadata: ConstructionMetadata,
    /// A suggested gas fee based on the current state of the network
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fee: Option<Vec<Amount>>,
}

/// Metadata required to construct a transaction
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionMetadata {
    /// Sequence number of the sending account
    pub sequence_number: u64,
    /// Maximum gas willing to pay for the transaction
    pub max_gas: u64,
    /// Multiplier e.g. how much each unit of gas is worth in the native coin
    pub gas_price_per_unit: u64,
}

/// Request to parse a signed or unsigned transaction into operations
///
/// This should be able to run without a running full node connection
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionParseRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionParseRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// Whether the transaction is a [`aptos_types::transaction::SignedTransaction`]
    /// or a [`aptos_types::transaction::RawTransaction`]
    pub signed: bool,
    /// A hex encoded, BCS encoded [`aptos_types::transaction::SignedTransaction`]
    /// or a [`aptos_types::transaction::RawTransaction`]
    pub transaction: String,
}

/// Response with operations in a transaction blob
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionParseResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionParseResponse {
    /// The set of [`Operation`] that happened during the transaction
    pub operations: Vec<Operation>,
    /// The signers of the transaction, if it was a [`aptos_types::transaction::SignedTransaction`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier_signers: Option<Vec<AccountIdentifier>>,
}

/// Request to build payloads from the operations to sign
///
/// This should be able to run without a running full node connection
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPayloadsRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPayloadsRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// The set of [`Operation`] that describes the [`InternalOperation`] to execute
    pub operations: Vec<Operation>,
    /// Required information for building a [`aptos_types::transaction::RawTransaction`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ConstructionMetadata>,
    /// Public keys of those who will sign the eventual [`aptos_types::transaction::SignedTransaction`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_keys: Option<Vec<PublicKey>>,
}

/// Response with generated payloads to be signed
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPayloadsResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPayloadsResponse {
    /// A hex encoded, BCS encoded [`aptos_types::transaction::RawTransaction`]
    /// containing the [`Operation`]s
    pub unsigned_transaction: String,
    /// Payloads describing who and what to sign
    pub payloads: Vec<SigningPayload>,
}

/// Request to get options for a [`ConstructionMetadataRequest`]
///
/// This should be able to run without a running full node connection
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPreprocessRequest.html)
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConstructionPreprocessRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// Operations that make up an `InternalOperation`
    pub operations: Vec<Operation>,
    /// Max gas price in the native coin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee: Option<Vec<Amount>>,
    /// Gas price per unit, must be an integer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fee_multiplier: Option<f64>,
}

/// Response for direct input into a [`ConstructionMetadataRequest`]
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPreprocessResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPreprocessResponse {
    /// Metadata to be sent verbatim to the Metadata API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<MetadataOptions>,
    /// List of who needs to be signing this transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_public_keys: Option<Vec<AccountIdentifier>>,
}

/// Request to submit a signed transaction
///
/// A running full node is required for this API
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionSubmitRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionSubmitRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// A hex encoded, BCS encoded [`aptos_types::transaction::SignedTransaction`]
    pub signed_transaction: String,
}

/// Response containing transaction identifier of submitted transaction
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionSubmitResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionSubmitResponse {
    /// Hash of the submitted [`aptos_types::transaction::SignedTransaction`]
    pub transaction_identifier: TransactionIdentifier,
}

/// Request for all transactions in mempool
///
/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
}

/// Response of all transactions in mempool
///
/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolResponse {
    /// Hash of the transactions in mempool
    pub transaction_identifiers: Vec<TransactionIdentifier>,
}

/// Request for a transaction in mempool by hash
///
/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolTransactionRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolTransactionRequest {
    /// Network identifier describing the blockchain and the chain id
    pub network_identifier: NetworkIdentifier,
    /// Hash of a transaction to lookup in mempool
    pub transaction_identifier: TransactionIdentifier,
}

/// Response of an estimate of the transaction in mempool
///
/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolTransactionResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolTransactionResponse {
    /// The transaction in mempool
    pub transaction: Transaction,
}

/// Metadata request for a placeholder when no other fields exist
///
/// [API Spec](https://www.rosetta-api.org/docs/models/MetadataRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MetadataRequest {}

/// Response of all networks that this endpoint supports
///
/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkListResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkListResponse {
    /// List of networks supported by this Rosetta instance
    pub network_identifiers: Vec<NetworkIdentifier>,
}

/// Response with all versioning and implementation specific fields
///
/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkOptionsResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkOptionsResponse {
    /// Software versions
    pub version: Version,
    /// Specifics about what is allowed on this server
    pub allow: Allow,
}

/// A generic request for network APIs
///
/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkRequest {
    pub network_identifier: NetworkIdentifier,
}

/// Response with information about the current network state
///
/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkStatusResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkStatusResponse {
    /// Current block identifier
    pub current_block_identifier: BlockIdentifier,
    /// Current block timestamp in milliseconds
    pub current_block_timestamp: u64,
    /// Genesis block
    pub genesis_block_identifier: BlockIdentifier,
    /// Oldest version that is available after pruning.  Assumed to be genesis block if not present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_block_identifier: Option<BlockIdentifier>,
    /// Sync status if a node needs to catch up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_status: Option<SyncStatus>,
    /// Connected peers
    pub peers: Vec<Peer>,
}

/// Response with a transaction that was hashed or submitted
///
/// [API Spec](https://www.rosetta-api.org/docs/models/TransactionIdentifierResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionIdentifierResponse {
    /// Hash of the transaction
    pub transaction_identifier: TransactionIdentifier,
}
