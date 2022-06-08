// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    AccountIdentifier, Allow, Amount, Block, BlockIdentifier, Currency, NetworkIdentifier,
    Operation, PartialBlockIdentifier, Peer, PublicKey, Signature, SigningPayload, SyncStatus,
    Transaction, TransactionIdentifier, Version,
};
use serde::{Deserialize, Serialize};

/// Request for an account's currency balance either now, or historically
///
/// [API Spec](https://www.rosetta-api.org/docs/models/AccountBalanceRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountBalanceRequest {
    pub network_identifier: NetworkIdentifier,
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
    pub block_identifier: BlockIdentifier,
    pub balances: Vec<Amount>,
}

/// Reqyest a block (version) on the account
///
/// With neither value for PartialBlockIdentifier, get the latest version
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockRequest {
    pub network_identifier: NetworkIdentifier,
    pub block_identifier: PartialBlockIdentifier,
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

///
///
/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionCombineRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionCombineRequest {
    pub network_identifier: NetworkIdentifier,
    pub unsigned_transaction: String,
    pub signatures: Vec<Signature>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionCombineResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionCombineResponse {
    pub signed_transaction: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionDeriveRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionDeriveRequest {
    pub network_identifier: NetworkIdentifier,
    pub public_key: PublicKey,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionDeriveResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionDeriveResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier: Option<AccountIdentifier>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionHashRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionHashRequest {
    pub network_identifier: NetworkIdentifier,
    pub signed_transaction: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionMetadataRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionMetadataRequest {
    pub network_identifier: NetworkIdentifier,
    pub options: MetadataOptions,
    pub public_keys: Vec<PublicKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MetadataOptions {
    /// The account that will construct the transaction
    pub sender_address: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionMetadataResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionMetadataResponse {
    pub metadata: ConstructionMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fee: Option<Vec<Amount>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionMetadata {
    pub chain_id: u8,
    pub sequence_number: u64,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionParseRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionParseRequest {
    pub network_identifier: NetworkIdentifier,
    pub signed: bool,
    pub transaction: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionParseResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionParseResponse {
    pub operations: Vec<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier_signers: Option<Vec<AccountIdentifier>>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPayloadsRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPayloadsRequest {
    pub network_identifier: NetworkIdentifier,
    pub operations: Vec<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ConstructionMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_keys: Option<Vec<PublicKey>>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPayloadsResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPayloadsResponse {
    pub unsigned_transaction: String,
    pub payloads: Vec<SigningPayload>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPreprocessRequest.html)
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConstructionPreprocessRequest {
    pub network_identifier: NetworkIdentifier,
    pub operations: Vec<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee: Option<Vec<Amount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fee_multiplier: Option<f64>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPreprocessResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPreprocessResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<MetadataOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_public_keys: Option<Vec<AccountIdentifier>>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionSubmitRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionSubmitRequest {
    pub network_identifier: NetworkIdentifier,
    pub signed_transaction: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionSubmitResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionSubmitResponse {
    pub transaction_identifier: TransactionIdentifier,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolRequest {
    pub network_identifier: NetworkIdentifier,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolResponse {
    pub transaction_identifiers: Vec<TransactionIdentifier>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolTransactionRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolTransactionRequest {
    pub network_identifier: NetworkIdentifier,
    pub transaction_identifier: TransactionIdentifier,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/MempoolTransactionResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MempoolTransactionResponse {
    pub transaction: Transaction,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/MetadataRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MetadataRequest {}

/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkListResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkListResponse {
    pub network_identifiers: Vec<NetworkIdentifier>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkOptionsResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkOptionsResponse {
    pub version: Version,
    pub allow: Allow,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkRequest {
    pub network_identifier: NetworkIdentifier,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkStatusResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkStatusResponse {
    pub current_block_identifier: BlockIdentifier,
    pub current_block_timestamp: u64,
    pub genesis_block_identifier: BlockIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_block_identifier: Option<BlockIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_status: Option<SyncStatus>,
    pub peers: Vec<Peer>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/TransactionIdentifierResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionIdentifierResponse {
    pub transaction_identifier: TransactionIdentifier,
}
