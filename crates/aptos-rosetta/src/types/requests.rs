// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    AccountIdentifier, Allow, Amount, Block, BlockIdentifier, Currency, NetworkIdentifier,
    Operation, PartialBlockIdentifier, Peer, PublicKey, Signature, SigningPayload, SyncStatus,
    Transaction, TransactionIdentifier, Version,
};
use serde::{Deserialize, Serialize};

/// [API Spec](https://www.rosetta-api.org/docs/models/AccountBalanceRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountBalanceRequest {
    pub network_identifier: NetworkIdentifier,
    pub account_identifier: AccountIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_identifier: Option<PartialBlockIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currencies: Option<Vec<Currency>>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/AccountBalanceResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountBalanceResponse {
    pub block_identifier: BlockIdentifier,
    pub balances: Vec<Amount>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BlockRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockRequest {
    pub network_identifier: NetworkIdentifier,
    pub block_identifier: PartialBlockIdentifier,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BlockResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockResponse {
    pub block: Option<Block>,
    pub other_transactions: Option<Vec<TransactionIdentifier>>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BlockTransactionRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockTransactionRequest {
    pub network_identifier: NetworkIdentifier,
    pub block_identifier: BlockIdentifier,
    pub transaction_identifier: TransactionIdentifier,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BlockTransactionResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockTransactionResponse {
    pub transaction: Transaction,
}

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
    pub account_identifier_signers: Option<Vec<AccountIdentifier>>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPayloadsRequest.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPayloadsRequest {
    pub network_identifier: NetworkIdentifier,
    pub operations: Vec<Operation>,
    pub metadata: Option<ConstructionMetadata>,
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
    pub max_fee: Option<Vec<Amount>>,
    pub suggested_fee_multiplier: Option<f64>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/ConstructionPreprocessResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConstructionPreprocessResponse {
    pub options: Option<MetadataOptions>,
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
    pub oldest_block_identifier: Option<BlockIdentifier>,
    pub sync_status: Option<SyncStatus>,
    pub peers: Vec<Peer>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/TransactionIdentifierResponse.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionIdentifierResponse {
    pub transaction_identifier: TransactionIdentifier,
}
