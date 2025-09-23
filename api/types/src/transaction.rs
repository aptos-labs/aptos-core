// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Address, AptosError, EntryFunctionId, EventGuid, HashValue, HexEncodedBytes,
    MoveModuleBytecode, MoveModuleId, MoveResource, MoveScriptBytecode, MoveStructTag, MoveType,
    MoveValue, VerifyInput, VerifyInputWithRecursion, U64,
};
use anyhow::{bail, Context as AnyhowContext, Result};
use aptos_crypto::{
    ed25519::{self, Ed25519PublicKey, ED25519_PUBLIC_KEY_LENGTH, ED25519_SIGNATURE_LENGTH},
    multi_ed25519::{self, MultiEd25519PublicKey, BITMAP_NUM_OF_BYTES, MAX_NUM_OF_KEYS},
    secp256k1_ecdsa, secp256r1_ecdsa,
    secp256r1_ecdsa::PUBLIC_KEY_LENGTH,
    ValidCryptoMaterial,
};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_metadata::BlockMetadata,
    block_metadata_ext::BlockMetadataExt,
    contract_event::{ContractEvent, EventWithVersion},
    dkg::{DKGTranscript, DKGTranscriptMetadata},
    function_info::FunctionInfo,
    jwks::{jwk::JWK, ProviderJWKs, QuorumCertifiedUpdate},
    keyless,
    transaction::{
        authenticator::{
            AbstractAuthenticator, AccountAuthenticator, AnyPublicKey, AnySignature, MultiKey,
            MultiKeyAuthenticator, SingleKeyAuthenticator, TransactionAuthenticator,
            MAX_NUM_OF_SIGS,
        },
        webauthn::{PartialAuthenticatorAssertionResponse, MAX_WEBAUTHN_SIGNATURE_BYTES},
        Script, SignedTransaction, TransactionOutput, TransactionWithProof,
    },
};
use bcs::to_bytes;
use once_cell::sync::Lazy;
use poem_openapi::{Object, Union};
use serde::{Deserialize, Serialize};
use std::{
    boxed::Box,
    convert::{From, Into, TryFrom, TryInto},
    fmt,
    str::FromStr,
};

static DUMMY_GUID: Lazy<EventGuid> = Lazy::new(|| EventGuid {
    creation_number: U64::from(0u64),
    account_address: Address::from(AccountAddress::ZERO),
});
static DUMMY_SEQUENCE_NUMBER: Lazy<U64> = Lazy::new(|| U64::from(0));

// Warning: Do not add a docstring to a field that uses a type in `derives.rs`,
// it will result in a change to the type representation. Read more about this
// issue here: https://github.com/poem-web/poem/issues/385.

// TODO: Add read_only / write_only (and their all variants) where appropriate.
// TODO: Investigate the use of discriminator_name, see https://github.com/poem-web/poem/issues/329.

/// Transaction data
///
/// This is a combination enum of an onchain transaction and a pending transaction.
/// When the transaction is still in mempool, it will be pending.  If it's been committed to the
/// chain, it will show up as OnChain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionData {
    /// A committed transaction
    OnChain(TransactionOnChainData),
    /// A transaction currently sitting in mempool
    Pending(Box<SignedTransaction>),
}

impl TransactionData {
    pub fn from_transaction_onchain_data(
        txn: TransactionOnChainData,
        latest_ledger_version: u64,
    ) -> Result<Self> {
        if txn.version > latest_ledger_version {
            match txn.transaction {
                aptos_types::transaction::Transaction::UserTransaction(txn) => {
                    Ok(Self::Pending(Box::new(txn)))
                },
                _ => bail!("convert non-user onchain transaction to pending shouldn't exist"),
            }
        } else {
            Ok(Self::OnChain(txn))
        }
    }
}

impl From<SignedTransaction> for TransactionData {
    fn from(txn: SignedTransaction) -> Self {
        Self::Pending(Box::new(txn))
    }
}

/// A committed transaction
///
/// This is a representation of the onchain payload, outputs, events, and proof of a transaction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionOnChainData {
    /// The ledger version of the transaction
    pub version: u64,
    /// The transaction submitted
    pub transaction: aptos_types::transaction::Transaction,
    /// Information about the transaction
    pub info: aptos_types::transaction::TransactionInfo,
    /// Events emitted by the transaction
    pub events: Vec<ContractEvent>,
    /// The accumulator root hash at this version
    pub accumulator_root_hash: aptos_crypto::HashValue,
    /// Final state of resources changed by the transaction
    pub changes: aptos_types::write_set::WriteSet,
}

impl From<(TransactionWithProof, aptos_crypto::HashValue)> for TransactionOnChainData {
    fn from((txn, accumulator_root_hash): (TransactionWithProof, aptos_crypto::HashValue)) -> Self {
        Self {
            version: txn.version,
            transaction: txn.transaction,
            info: txn.proof.transaction_info,
            events: txn.events.unwrap_or_default(),
            accumulator_root_hash,
            changes: Default::default(),
        }
    }
}

impl
    From<(
        TransactionWithProof,
        aptos_crypto::HashValue,
        &TransactionOutput,
    )> for TransactionOnChainData
{
    fn from(
        (txn, accumulator_root_hash, txn_output): (
            TransactionWithProof,
            aptos_crypto::HashValue,
            &TransactionOutput,
        ),
    ) -> Self {
        Self {
            version: txn.version,
            transaction: txn.transaction,
            info: txn.proof.transaction_info,
            events: txn.events.unwrap_or_default(),
            accumulator_root_hash,
            changes: txn_output.write_set().clone(),
        }
    }
}

impl
    From<(
        u64,
        aptos_types::transaction::Transaction,
        aptos_types::transaction::TransactionInfo,
        Vec<ContractEvent>,
        aptos_crypto::HashValue,
        aptos_types::write_set::WriteSet,
    )> for TransactionOnChainData
{
    fn from(
        (version, transaction, info, events, accumulator_root_hash, write_set): (
            u64,
            aptos_types::transaction::Transaction,
            aptos_types::transaction::TransactionInfo,
            Vec<ContractEvent>,
            aptos_crypto::HashValue,
            aptos_types::write_set::WriteSet,
        ),
    ) -> Self {
        Self {
            version,
            transaction,
            info,
            events,
            accumulator_root_hash,
            changes: write_set,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
#[serde(tag = "type")]
pub struct TransactionSummary {
    pub sender: Address,
    pub version: U64,
    pub transaction_hash: HashValue,
    pub replay_protector: ReplayProtector,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum ReplayProtector {
    Nonce(U64),
    SequenceNumber(U64),
}

/// Enum of the different types of transactions in Aptos
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum Transaction {
    PendingTransaction(PendingTransaction),
    UserTransaction(UserTransaction),
    GenesisTransaction(GenesisTransaction),
    BlockMetadataTransaction(BlockMetadataTransaction),
    StateCheckpointTransaction(StateCheckpointTransaction),
    BlockEpilogueTransaction(BlockEpilogueTransaction),
    ValidatorTransaction(ValidatorTransaction),
}

impl Transaction {
    pub fn timestamp(&self) -> u64 {
        match self {
            Transaction::UserTransaction(txn) => txn.timestamp.0,
            Transaction::BlockMetadataTransaction(txn) => txn.timestamp.0,
            Transaction::PendingTransaction(_) => 0,
            Transaction::GenesisTransaction(_) => 0,
            Transaction::StateCheckpointTransaction(txn) => txn.timestamp.0,
            Transaction::BlockEpilogueTransaction(txn) => txn.timestamp.0,
            Transaction::ValidatorTransaction(txn) => txn.timestamp().0,
        }
    }

    pub fn version(&self) -> Option<u64> {
        match self {
            Transaction::UserTransaction(txn) => Some(txn.info.version.into()),
            Transaction::BlockMetadataTransaction(txn) => Some(txn.info.version.into()),
            Transaction::PendingTransaction(_) => None,
            Transaction::GenesisTransaction(txn) => Some(txn.info.version.into()),
            Transaction::StateCheckpointTransaction(txn) => Some(txn.info.version.into()),
            Transaction::BlockEpilogueTransaction(txn) => Some(txn.info.version.into()),
            Transaction::ValidatorTransaction(txn) => Some(txn.transaction_info().version.into()),
        }
    }

    pub fn success(&self) -> bool {
        match self {
            Transaction::UserTransaction(txn) => txn.info.success,
            Transaction::BlockMetadataTransaction(txn) => txn.info.success,
            Transaction::PendingTransaction(_txn) => false,
            Transaction::GenesisTransaction(txn) => txn.info.success,
            Transaction::StateCheckpointTransaction(txn) => txn.info.success,
            Transaction::BlockEpilogueTransaction(txn) => txn.info.success,
            Transaction::ValidatorTransaction(txn) => txn.transaction_info().success,
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Transaction::PendingTransaction(_))
    }

    pub fn vm_status(&self) -> String {
        match self {
            Transaction::UserTransaction(txn) => txn.info.vm_status.clone(),
            Transaction::BlockMetadataTransaction(txn) => txn.info.vm_status.clone(),
            Transaction::PendingTransaction(_txn) => "pending".to_owned(),
            Transaction::GenesisTransaction(txn) => txn.info.vm_status.clone(),
            Transaction::StateCheckpointTransaction(txn) => txn.info.vm_status.clone(),
            Transaction::BlockEpilogueTransaction(txn) => txn.info.vm_status.clone(),
            Transaction::ValidatorTransaction(txn) => txn.transaction_info().vm_status.clone(),
        }
    }

    pub fn type_str(&self) -> &'static str {
        match self {
            Transaction::PendingTransaction(_) => "pending_transaction",
            Transaction::UserTransaction(_) => "user_transaction",
            Transaction::GenesisTransaction(_) => "genesis_transaction",
            Transaction::BlockMetadataTransaction(_) => "block_metadata_transaction",
            Transaction::StateCheckpointTransaction(_) => "state_checkpoint_transaction",
            Transaction::BlockEpilogueTransaction(_) => "block_epilogue_transaction",
            Transaction::ValidatorTransaction(vt) => vt.type_str(),
        }
    }

    pub fn transaction_info(&self) -> anyhow::Result<&TransactionInfo> {
        Ok(match self {
            Transaction::UserTransaction(txn) => &txn.info,
            Transaction::BlockMetadataTransaction(txn) => &txn.info,
            Transaction::PendingTransaction(_txn) => {
                bail!("pending transaction does not have TransactionInfo")
            },
            Transaction::GenesisTransaction(txn) => &txn.info,
            Transaction::StateCheckpointTransaction(txn) => &txn.info,
            Transaction::BlockEpilogueTransaction(txn) => &txn.info,
            Transaction::ValidatorTransaction(txn) => txn.transaction_info(),
        })
    }
}

// TODO: Remove this when we cut over to the new API fully.
impl From<(SignedTransaction, TransactionPayload)> for Transaction {
    fn from((txn, payload): (SignedTransaction, TransactionPayload)) -> Self {
        Transaction::PendingTransaction(PendingTransaction {
            request: (&txn, payload).into(),
            hash: txn.committed_hash().into(),
        })
    }
}

impl
    From<(
        &SignedTransaction,
        TransactionInfo,
        TransactionPayload,
        Vec<Event>,
        u64,
    )> for Transaction
{
    fn from(
        (txn, info, payload, events, timestamp): (
            &SignedTransaction,
            TransactionInfo,
            TransactionPayload,
            Vec<Event>,
            u64,
        ),
    ) -> Self {
        Transaction::UserTransaction(UserTransaction {
            info,
            request: (txn, payload).into(),
            events,
            timestamp: timestamp.into(),
        })
    }
}

impl From<(TransactionInfo, WriteSetPayload, Vec<Event>)> for Transaction {
    fn from((info, payload, events): (TransactionInfo, WriteSetPayload, Vec<Event>)) -> Self {
        Transaction::GenesisTransaction(GenesisTransaction {
            info,
            payload: GenesisPayload::WriteSetPayload(payload),
            events,
        })
    }
}

impl From<(&SignedTransaction, TransactionPayload)> for UserTransactionRequest {
    fn from((txn, payload): (&SignedTransaction, TransactionPayload)) -> Self {
        Self {
            sender: txn.sender().into(),
            sequence_number: txn.sequence_number().into(),
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs().into(),
            signature: Some(txn.authenticator().into()),
            payload,
            replay_protection_nonce: txn.replay_protector().get_nonce().map(|nonce| nonce.into()),
        }
    }
}

/// Information related to how a transaction affected the state of the blockchain
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct TransactionInfo {
    pub version: U64,
    pub hash: HashValue,
    pub state_change_hash: HashValue,
    pub event_root_hash: HashValue,
    pub state_checkpoint_hash: Option<HashValue>,
    pub gas_used: U64,
    /// Whether the transaction was successful
    pub success: bool,
    /// The VM status of the transaction, can tell useful information in a failure
    pub vm_status: String,
    pub accumulator_root_hash: HashValue,
    /// Final state of resources changed by the transaction
    pub changes: Vec<WriteSetChange>,
    /// Block height that the transaction belongs in, this field will not be present through the API
    #[oai(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<U64>,
    /// Epoch of the transaction belongs in, this field will not be present through the API
    #[oai(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<U64>,
}

/// A transaction waiting in mempool
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct PendingTransaction {
    pub hash: HashValue,
    #[serde(flatten)]
    #[oai(flatten)]
    pub request: UserTransactionRequest,
}

impl From<(SignedTransaction, TransactionPayload)> for PendingTransaction {
    fn from((txn, payload): (SignedTransaction, TransactionPayload)) -> Self {
        PendingTransaction {
            request: (&txn, payload).into(),
            hash: txn.committed_hash().into(),
        }
    }
}
/// A transaction submitted by a user to change the state of the blockchain
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct UserTransaction {
    #[serde(flatten)]
    #[oai(flatten)]
    pub info: TransactionInfo,
    #[serde(flatten)]
    #[oai(flatten)]
    pub request: UserTransactionRequest,
    /// Events generated by the transaction
    pub events: Vec<Event>,
    pub timestamp: U64,
}

/// A state checkpoint transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct StateCheckpointTransaction {
    #[serde(flatten)]
    #[oai(flatten)]
    pub info: TransactionInfo,
    pub timestamp: U64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct BlockEndInfo {
    pub block_gas_limit_reached: bool,
    pub block_output_limit_reached: bool,
    pub block_effective_block_gas_units: u64,
    pub block_approx_output_size: u64,
}

/// A block epilogue transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct BlockEpilogueTransaction {
    #[serde(flatten)]
    #[oai(flatten)]
    pub info: TransactionInfo,
    pub timestamp: U64,
    pub block_end_info: Option<BlockEndInfo>,
}

/// A request to submit a transaction
///
/// This requires a transaction and a signature of it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct SubmitTransactionRequest {
    #[serde(flatten)]
    #[oai(flatten)]
    pub user_transaction_request: UserTransactionRequestInner,
    pub signature: TransactionSignature,
}

impl VerifyInput for SubmitTransactionRequest {
    fn verify(&self) -> anyhow::Result<()> {
        self.user_transaction_request.verify()?;
        self.signature.verify()
    }
}

/// Batch transaction submission result
///
/// Tells which transactions failed
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TransactionsBatchSubmissionResult {
    /// Summary of the failed transactions
    pub transaction_failures: Vec<TransactionsBatchSingleSubmissionFailure>,
}

/// Information telling which batch submission transactions failed
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TransactionsBatchSingleSubmissionFailure {
    pub error: AptosError,
    /// The index of which transaction failed, same as submission order
    pub transaction_index: usize,
}

// TODO: Rename this to remove the Inner when we cut over.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct UserTransactionRequestInner {
    pub sender: Address,
    pub sequence_number: U64,
    pub max_gas_amount: U64,
    pub gas_unit_price: U64,
    pub expiration_timestamp_secs: U64,
    pub payload: TransactionPayload,
    pub replay_protection_nonce: Option<U64>,
}

impl VerifyInput for UserTransactionRequestInner {
    fn verify(&self) -> anyhow::Result<()> {
        self.payload.verify()
    }
}

// TODO: Remove this when we cut over.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct UserTransactionRequest {
    pub sender: Address,
    pub sequence_number: U64,
    pub max_gas_amount: U64,
    pub gas_unit_price: U64,
    pub expiration_timestamp_secs: U64,
    pub payload: TransactionPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<TransactionSignature>,
    pub replay_protection_nonce: Option<U64>,
}

impl UserTransactionRequest {
    pub fn replay_protector(&self) -> aptos_types::transaction::ReplayProtector {
        if let Some(nonce) = self.replay_protection_nonce {
            aptos_types::transaction::ReplayProtector::Nonce(nonce.0)
        } else {
            aptos_types::transaction::ReplayProtector::SequenceNumber(self.sequence_number.0)
        }
    }
}

/// Request to create signing messages
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct UserCreateSigningMessageRequest {
    #[serde(flatten)]
    #[oai(flatten)]
    pub transaction: UserTransactionRequest,
    /// Secondary signer accounts of the request for Multi-agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_signers: Option<Vec<Address>>,
}

/// Request to encode a submission
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct EncodeSubmissionRequest {
    #[serde(flatten)]
    #[oai(flatten)]
    pub transaction: UserTransactionRequestInner,
    /// Secondary signer accounts of the request for Multi-agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_signers: Option<Vec<Address>>,
}

impl VerifyInput for EncodeSubmissionRequest {
    fn verify(&self) -> anyhow::Result<()> {
        self.transaction.verify()
    }
}

/// The genesis transaction
///
/// This only occurs at the genesis transaction (version 0)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct GenesisTransaction {
    #[serde(flatten)]
    #[oai(flatten)]
    pub info: TransactionInfo,
    pub payload: GenesisPayload,
    /// Events emitted during genesis
    pub events: Vec<Event>,
}

/// A block metadata transaction
///
/// This signifies the beginning of a block, and contains information
/// about the specific block
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct BlockMetadataTransaction {
    #[serde(flatten)]
    #[oai(flatten)]
    pub info: TransactionInfo,
    pub id: HashValue,
    pub epoch: U64,
    pub round: U64,
    /// The events emitted at the block creation
    pub events: Vec<Event>,
    /// Previous block votes
    pub previous_block_votes_bitvec: Vec<u8>,
    pub proposer: Address,
    /// The indices of the proposers who failed to propose
    pub failed_proposer_indices: Vec<u32>,
    pub timestamp: U64,

    /// If some, it means the internal txn type is `aptos_types::transaction::Transaction::BlockMetadataExt`.
    /// Otherwise, it is `aptos_types::transaction::Transaction::BlockMetadata`.
    ///
    /// NOTE: we could have introduced a new APT txn type to represent the corresponding internal type,
    /// but that is a breaking change to the ecosystem.
    ///
    /// NOTE: `oai` does not support `flatten` together with `skip_serializing_if`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[oai(default, skip_serializing_if = "Option::is_none")]
    pub block_metadata_extension: Option<BlockMetadataExtension>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct BlockMetadataExtensionEmpty {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct BlockMetadataExtensionRandomness {
    randomness: Option<HexEncodedBytes>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum BlockMetadataExtension {
    V0(BlockMetadataExtensionEmpty),
    V1(BlockMetadataExtensionRandomness),
}

impl BlockMetadataExtension {
    pub fn from_internal_txn(txn: &BlockMetadataExt) -> Self {
        match txn {
            BlockMetadataExt::V0(_) => Self::V0(BlockMetadataExtensionEmpty {}),
            BlockMetadataExt::V1(payload) => Self::V1(BlockMetadataExtensionRandomness {
                randomness: payload
                    .randomness
                    .as_ref()
                    .map(|pr| HexEncodedBytes::from(pr.randomness_cloned())),
            }),
        }
    }
}

impl BlockMetadataTransaction {
    pub fn from_internal(
        internal: BlockMetadata,
        info: TransactionInfo,
        events: Vec<Event>,
    ) -> Self {
        Self {
            info,
            id: internal.id().into(),
            epoch: internal.epoch().into(),
            round: internal.round().into(),
            events,
            previous_block_votes_bitvec: internal.previous_block_votes_bitvec().clone(),
            proposer: internal.proposer().into(),
            failed_proposer_indices: internal.failed_proposer_indices().clone(),
            timestamp: internal.timestamp_usecs().into(),
            block_metadata_extension: None,
        }
    }

    pub fn from_internal_ext(
        internal: BlockMetadataExt,
        info: TransactionInfo,
        events: Vec<Event>,
    ) -> Self {
        Self {
            info,
            id: internal.id().into(),
            epoch: internal.epoch().into(),
            round: internal.round().into(),
            events,
            previous_block_votes_bitvec: internal.previous_block_votes_bitvec().clone(),
            proposer: internal.proposer().into(),
            failed_proposer_indices: internal.failed_proposer_indices().clone(),
            timestamp: internal.timestamp_usecs().into(),
            block_metadata_extension: Some(BlockMetadataExtension::from_internal_txn(&internal)),
        }
    }

    pub fn type_str(&self) -> &'static str {
        match self.block_metadata_extension {
            None => "block_metadata_transaction",
            Some(BlockMetadataExtension::V0(_)) => "block_metadata_ext_transaction__v0",
            Some(BlockMetadataExtension::V1(_)) => "block_metadata_ext_transaction__v1",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "validator_transaction_type", rename_all = "snake_case")]
#[oai(
    one_of,
    discriminator_name = "validator_transaction_type",
    rename_all = "snake_case"
)]
pub enum ValidatorTransaction {
    ObservedJwkUpdate(JWKUpdateTransaction),
    DkgResult(DKGResultTransaction),
}

impl ValidatorTransaction {
    pub fn type_str(&self) -> &'static str {
        match self {
            ValidatorTransaction::ObservedJwkUpdate(_) => {
                "validator_transaction__observed_jwk_update"
            },
            ValidatorTransaction::DkgResult(_) => "validator_transaction__dkg_result",
        }
    }

    pub fn transaction_info(&self) -> &TransactionInfo {
        match self {
            ValidatorTransaction::ObservedJwkUpdate(t) => &t.info,
            ValidatorTransaction::DkgResult(t) => &t.info,
        }
    }

    pub fn transaction_info_mut(&mut self) -> &mut TransactionInfo {
        match self {
            ValidatorTransaction::ObservedJwkUpdate(t) => &mut t.info,
            ValidatorTransaction::DkgResult(t) => &mut t.info,
        }
    }

    pub fn timestamp(&self) -> U64 {
        match self {
            ValidatorTransaction::ObservedJwkUpdate(t) => t.timestamp,
            ValidatorTransaction::DkgResult(t) => t.timestamp,
        }
    }

    pub fn events(&self) -> &[Event] {
        match self {
            ValidatorTransaction::ObservedJwkUpdate(t) => &t.events,
            ValidatorTransaction::DkgResult(t) => &t.events,
        }
    }
}

impl
    From<(
        aptos_types::validator_txn::ValidatorTransaction,
        TransactionInfo,
        Vec<Event>,
        u64,
    )> for ValidatorTransaction
{
    fn from(
        (txn, info, events, timestamp): (
            aptos_types::validator_txn::ValidatorTransaction,
            TransactionInfo,
            Vec<Event>,
            u64,
        ),
    ) -> Self {
        match txn {
            aptos_types::validator_txn::ValidatorTransaction::DKGResult(dkg_transcript) => {
                Self::DkgResult(DKGResultTransaction {
                    info,
                    events,
                    timestamp: U64::from(timestamp),
                    dkg_transcript: dkg_transcript.into(),
                })
            },
            aptos_types::validator_txn::ValidatorTransaction::ObservedJWKUpdate(
                quorum_certified_update,
            ) => Self::ObservedJwkUpdate(JWKUpdateTransaction {
                info,
                events,
                timestamp: U64::from(timestamp),
                quorum_certified_update: quorum_certified_update.into(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct JWKUpdateTransaction {
    #[serde(flatten)]
    #[oai(flatten)]
    pub info: TransactionInfo,
    pub events: Vec<Event>,
    pub timestamp: U64,
    pub quorum_certified_update: ExportedQuorumCertifiedUpdate,
}

/// A more API-friendly representation of the on-chain `aptos_types::jwks::QuorumCertifiedUpdate`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct ExportedQuorumCertifiedUpdate {
    pub update: ExportedProviderJWKs,
    pub multi_sig: ExportedAggregateSignature,
}

impl From<QuorumCertifiedUpdate> for ExportedQuorumCertifiedUpdate {
    fn from(value: QuorumCertifiedUpdate) -> Self {
        let QuorumCertifiedUpdate { update, multi_sig } = value;
        Self {
            update: update.into(),
            multi_sig: multi_sig.into(),
        }
    }
}

/// A more API-friendly representation of the on-chain `aptos_types::aggregate_signature::AggregateSignature`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct ExportedAggregateSignature {
    pub signer_indices: Vec<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<HexEncodedBytes>,
}

impl From<AggregateSignature> for ExportedAggregateSignature {
    fn from(value: AggregateSignature) -> Self {
        Self {
            signer_indices: value.get_signers_bitvec().iter_ones().collect(),
            sig: value
                .sig()
                .as_ref()
                .map(|s| HexEncodedBytes::from(s.to_bytes().to_vec())),
        }
    }
}

/// A more API-friendly representation of the on-chain `aptos_types::jwks::ProviderJWKs`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct ExportedProviderJWKs {
    pub issuer: String,
    pub version: u64,
    pub jwks: Vec<JWK>,
}

impl From<ProviderJWKs> for ExportedProviderJWKs {
    fn from(value: ProviderJWKs) -> Self {
        let ProviderJWKs {
            issuer,
            version,
            jwks,
        } = value;
        Self {
            issuer: String::from_utf8(issuer).unwrap_or("non_utf8_issuer".to_string()),
            version,
            jwks: jwks.iter().map(|on_chain_jwk|{
                JWK::try_from(on_chain_jwk).expect("conversion from on-chain representation to human-friendly representation should work")
            }).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DKGResultTransaction {
    #[serde(flatten)]
    #[oai(flatten)]
    pub info: TransactionInfo,
    pub events: Vec<Event>,
    pub timestamp: U64,
    pub dkg_transcript: ExportedDKGTranscript,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct ExportedDKGTranscript {
    pub epoch: U64,
    pub author: Address,
    pub payload: HexEncodedBytes,
}

impl From<DKGTranscript> for ExportedDKGTranscript {
    fn from(value: DKGTranscript) -> Self {
        let DKGTranscript {
            metadata,
            transcript_bytes,
        } = value;
        let DKGTranscriptMetadata { epoch, author } = metadata;
        Self {
            epoch: epoch.into(),
            author: author.into(),
            payload: HexEncodedBytes::from(transcript_bytes),
        }
    }
}

/// An event from a transaction
#[derive(Clone, Debug, Deserialize, Eq, Object, PartialEq, Serialize)]
pub struct Event {
    // The globally unique identifier of this event stream.
    pub guid: EventGuid,
    // The sequence number of the event
    pub sequence_number: U64,
    #[serde(rename = "type")]
    #[oai(rename = "type")]
    pub typ: MoveType,
    /// The JSON representation of the event
    pub data: serde_json::Value,
}

impl From<(&ContractEvent, serde_json::Value)> for Event {
    fn from((event, data): (&ContractEvent, serde_json::Value)) -> Self {
        match event {
            ContractEvent::V1(v1) => Self {
                guid: (*v1.key()).into(),
                sequence_number: v1.sequence_number().into(),
                typ: v1.type_tag().into(),
                data,
            },
            ContractEvent::V2(v2) => Self {
                guid: *DUMMY_GUID,
                sequence_number: *DUMMY_SEQUENCE_NUMBER,
                typ: v2.type_tag().into(),
                data,
            },
        }
    }
}

/// An event from a transaction with a version
#[derive(Clone, Debug, Deserialize, Eq, Object, PartialEq, Serialize)]
pub struct VersionedEvent {
    pub version: U64,
    // The globally unique identifier of this event stream.
    pub guid: EventGuid,
    // The sequence number of the event
    pub sequence_number: U64,
    #[serde(rename = "type")]
    #[oai(rename = "type")]
    pub typ: MoveType,
    /// The JSON representation of the event
    pub data: serde_json::Value,
}

impl From<(&EventWithVersion, serde_json::Value)> for VersionedEvent {
    fn from((event, data): (&EventWithVersion, serde_json::Value)) -> Self {
        match &event.event {
            ContractEvent::V1(v1) => Self {
                version: event.transaction_version.into(),
                guid: (*v1.key()).into(),
                sequence_number: v1.sequence_number().into(),
                typ: v1.type_tag().into(),
                data,
            },
            ContractEvent::V2(v2) => Self {
                version: event.transaction_version.into(),
                guid: *DUMMY_GUID,
                sequence_number: *DUMMY_SEQUENCE_NUMBER,
                typ: v2.type_tag().into(),
                data,
            },
        }
    }
}

/// The writeset payload of the Genesis transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum GenesisPayload {
    WriteSetPayload(WriteSetPayload),
}

/// An enum of the possible transaction payloads
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum TransactionPayload {
    EntryFunctionPayload(EntryFunctionPayload),
    ScriptPayload(ScriptPayload),
    // Deprecated. We cannot remove the enum variant because it breaks the
    // ordering, unfortunately.
    ModuleBundlePayload(DeprecatedModuleBundlePayload),
    MultisigPayload(MultisigPayload),
}

impl VerifyInput for TransactionPayload {
    fn verify(&self) -> anyhow::Result<()> {
        match self {
            TransactionPayload::EntryFunctionPayload(inner) => inner.verify(),
            TransactionPayload::ScriptPayload(inner) => inner.verify(),
            TransactionPayload::MultisigPayload(inner) => inner.verify(),

            // Deprecated.
            TransactionPayload::ModuleBundlePayload(_) => {
                bail!("Module bundle payload has been removed")
            },
        }
    }
}

// We cannot remove enum variant, but at least we can remove the logic
// and keep a deprecate name here to avoid further usage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DeprecatedModuleBundlePayload;

/// Payload which runs a single entry function
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct EntryFunctionPayload {
    pub function: EntryFunctionId,
    /// Type arguments of the function
    pub type_arguments: Vec<MoveType>,
    /// Arguments of the function
    pub arguments: Vec<serde_json::Value>,
}

impl VerifyInput for EntryFunctionPayload {
    fn verify(&self) -> anyhow::Result<()> {
        self.function.verify()?;
        for type_arg in self.type_arguments.iter() {
            type_arg.verify(0)?;
        }
        Ok(())
    }
}

/// Payload which runs a script that can run multiple functions
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct ScriptPayload {
    pub code: MoveScriptBytecode,
    /// Type arguments of the function
    pub type_arguments: Vec<MoveType>,
    /// Arguments of the function
    pub arguments: Vec<serde_json::Value>,
}

impl VerifyInput for ScriptPayload {
    fn verify(&self) -> anyhow::Result<()> {
        for type_arg in self.type_arguments.iter() {
            type_arg.verify(0)?;
        }
        Ok(())
    }
}

impl TryFrom<Script> for ScriptPayload {
    type Error = anyhow::Error;

    fn try_from(script: Script) -> anyhow::Result<Self> {
        let (code, ty_args, args) = script.into_inner();
        Ok(Self {
            code: MoveScriptBytecode::new(code).try_parse_abi(),
            type_arguments: ty_args.iter().map(|arg| arg.into()).collect(),
            arguments: args
                .into_iter()
                .map(|arg| MoveValue::from(arg).json())
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

// We use an enum here for extensibility so we can add Script payload support
// in the future for example.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum MultisigTransactionPayload {
    EntryFunctionPayload(EntryFunctionPayload),
}

/// A multisig transaction that allows an owner of a multisig account to execute a pre-approved
/// transaction as the multisig account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MultisigPayload {
    pub multisig_address: Address,

    // Transaction payload is optional if already stored on chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub transaction_payload: Option<MultisigTransactionPayload>,
}

impl VerifyInput for MultisigPayload {
    fn verify(&self) -> anyhow::Result<()> {
        if let Some(payload) = &self.transaction_payload {
            match payload {
                MultisigTransactionPayload::EntryFunctionPayload(entry_function) => {
                    entry_function.function.verify()?;
                    for type_arg in entry_function.type_arguments.iter() {
                        type_arg.verify(0)?;
                    }
                },
            }
        }

        Ok(())
    }
}

/// A writeset payload, used only for genesis
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct WriteSetPayload {
    pub write_set: WriteSet,
}

/// The associated writeset with a payload
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum WriteSet {
    ScriptWriteSet(ScriptWriteSet),
    DirectWriteSet(DirectWriteSet),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct ScriptWriteSet {
    pub execute_as: Address,
    pub script: ScriptPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DirectWriteSet {
    pub changes: Vec<WriteSetChange>,
    pub events: Vec<Event>,
}

/// A final state change of a transaction on a resource or module
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum WriteSetChange {
    DeleteModule(DeleteModule),
    DeleteResource(DeleteResource),
    DeleteTableItem(DeleteTableItem),
    WriteModule(WriteModule),
    WriteResource(WriteResource),
    WriteTableItem(WriteTableItem),
}

/// Delete a module
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DeleteModule {
    pub address: Address,
    /// State key hash
    pub state_key_hash: String,
    pub module: MoveModuleId,
}

/// Delete a resource
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DeleteResource {
    pub address: Address,
    /// State key hash
    pub state_key_hash: String,
    pub resource: MoveStructTag,
}

/// Delete a table item
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DeleteTableItem {
    pub state_key_hash: String,
    pub handle: HexEncodedBytes,
    pub key: HexEncodedBytes,
    // This is optional, and only possible to populate if the table indexer is enabled for this node
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub data: Option<DeletedTableData>,
}

/// Write a new module or update an existing one
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct WriteModule {
    pub address: Address,
    /// State key hash
    pub state_key_hash: String,
    pub data: MoveModuleBytecode,
}

/// Write a resource or update an existing one
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct WriteResource {
    pub address: Address,
    /// State key hash
    pub state_key_hash: String,
    pub data: MoveResource,
}

/// Decoded table data
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DecodedTableData {
    /// Key of table in JSON
    pub key: serde_json::Value,
    /// Type of key
    pub key_type: String,
    /// Value of table in JSON
    pub value: serde_json::Value,
    /// Type of value
    pub value_type: String,
}

/// Deleted table data
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DeletedTableData {
    /// Deleted key
    pub key: serde_json::Value,
    /// Deleted key type
    pub key_type: String,
}

/// Change set to write a table item
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct WriteTableItem {
    pub state_key_hash: String,
    pub handle: HexEncodedBytes,
    pub key: HexEncodedBytes,
    pub value: HexEncodedBytes,
    // This is optional, and only possible to populate if the table indexer is enabled for this node
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub data: Option<DecodedTableData>,
}

impl WriteSetChange {
    pub fn type_str(&self) -> &'static str {
        match self {
            WriteSetChange::DeleteModule { .. } => "delete_module",
            WriteSetChange::DeleteResource { .. } => "delete_resource",
            WriteSetChange::DeleteTableItem { .. } => "delete_table_item",
            WriteSetChange::WriteModule { .. } => "write_module",
            WriteSetChange::WriteResource { .. } => "write_resource",
            WriteSetChange::WriteTableItem { .. } => "write_table_item",
        }
    }
}

/// An enum representing the different transaction signatures available
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum TransactionSignature {
    Ed25519Signature(Ed25519Signature),
    MultiEd25519Signature(MultiEd25519Signature),
    MultiAgentSignature(MultiAgentSignature),
    FeePayerSignature(FeePayerSignature),
    SingleSender(AccountSignature),
    NoAccountSignature(NoAccountSignature),
}

impl VerifyInput for TransactionSignature {
    fn verify(&self) -> anyhow::Result<()> {
        match self {
            TransactionSignature::Ed25519Signature(inner) => inner.verify(),
            TransactionSignature::MultiEd25519Signature(inner) => inner.verify(),
            TransactionSignature::MultiAgentSignature(inner) => inner.verify(),
            TransactionSignature::FeePayerSignature(inner) => inner.verify(),
            TransactionSignature::SingleSender(inner) => inner.verify(),
            TransactionSignature::NoAccountSignature(inner) => inner.verify(),
        }
    }
}

impl TryFrom<&TransactionSignature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(ts: &TransactionSignature) -> anyhow::Result<Self> {
        Ok(match ts {
            TransactionSignature::Ed25519Signature(sig) => sig.try_into()?,
            TransactionSignature::MultiEd25519Signature(sig) => sig.try_into()?,
            TransactionSignature::MultiAgentSignature(sig) => sig.try_into()?,
            TransactionSignature::FeePayerSignature(sig) => sig.try_into()?,
            TransactionSignature::SingleSender(sig) => {
                TransactionAuthenticator::single_sender(sig.try_into()?)
            },
            TransactionSignature::NoAccountSignature(sig) => sig.try_into()?,
        })
    }
}

/// A single Ed25519 signature
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct Ed25519Signature {
    pub public_key: HexEncodedBytes,
    pub signature: HexEncodedBytes,
}

impl VerifyInput for Ed25519Signature {
    fn verify(&self) -> anyhow::Result<()> {
        let public_key_len = self.public_key.inner().len();
        let signature_len = self.signature.inner().len();
        if public_key_len != ED25519_PUBLIC_KEY_LENGTH {
            bail!(
                "Ed25519 signature's public key is an invalid number of bytes, should be {} bytes but found {}",
                ED25519_PUBLIC_KEY_LENGTH, public_key_len
            )
        } else if signature_len != ED25519_SIGNATURE_LENGTH {
            bail!(
                "Ed25519 signature length is an invalid number of bytes, should be {} bytes but found {}",
                ED25519_SIGNATURE_LENGTH, signature_len
            )
        } else {
            // TODO: Check if they match / parse correctly?
            Ok(())
        }
    }
}

impl TryFrom<&Ed25519Signature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &Ed25519Signature) -> Result<Self, Self::Error> {
        Ok(TransactionAuthenticator::ed25519(
            value
                .public_key
                .inner()
                .try_into()
                .context("Failed to parse given public_key bytes as a Ed25519PublicKey")?,
            value
                .signature
                .inner()
                .try_into()
                .context("Failed to parse given signature as a Ed25519Signature")?,
        ))
    }
}

impl TryFrom<&Ed25519Signature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &Ed25519Signature) -> Result<Self, Self::Error> {
        Ok(AccountAuthenticator::ed25519(
            value
                .public_key
                .inner()
                .try_into()
                .context("Failed to parse given public_key bytes as a Ed25519PublicKey")?,
            value
                .signature
                .inner()
                .try_into()
                .context("Failed to parse given signature as a Ed25519Signature")?,
        ))
    }
}

/// A Ed25519 multi-sig signature
///
/// This allows k-of-n signing for a transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MultiEd25519Signature {
    /// The public keys for the Ed25519 signature
    pub public_keys: Vec<HexEncodedBytes>,
    /// Signature associated with the public keys in the same order
    pub signatures: Vec<HexEncodedBytes>,
    /// The number of signatures required for a successful transaction
    pub threshold: u8,
    pub bitmap: HexEncodedBytes,
}

impl VerifyInput for MultiEd25519Signature {
    fn verify(&self) -> anyhow::Result<()> {
        if self.public_keys.is_empty() {
            bail!("MultiEd25519 signature has no public keys")
        } else if self.signatures.is_empty() {
            bail!("MultiEd25519 signature has no signatures")
        } else if self.public_keys.len() > MAX_NUM_OF_KEYS {
            bail!(
                "MultiEd25519 signature has over the maximum number of public keys {}",
                MAX_NUM_OF_KEYS
            )
        } else if self.signatures.len() > MAX_NUM_OF_SIGS {
            bail!(
                "MultiEd25519 signature has over the maximum number of signatures {}",
                MAX_NUM_OF_SIGS
            )
        } else if self.public_keys.len() != self.signatures.len() {
            bail!(
                "MultiEd25519 signature does not have the same number of signatures as public keys"
            )
        } else if self.signatures.len() < self.threshold as usize {
            bail!("MultiEd25519 signature does not have enough signatures to pass the threshold")
        } else if self.threshold == 0 {
            bail!("MultiEd25519 signature threshold must be greater than 0")
        }
        for signature in self.signatures.iter() {
            if signature.inner().len() != ED25519_SIGNATURE_LENGTH {
                bail!("MultiEd25519 signature has a signature with the wrong signature length")
            }
        }
        for public_key in self.public_keys.iter() {
            if public_key.inner().len() != ED25519_PUBLIC_KEY_LENGTH {
                bail!("MultiEd25519 signature has a public key with the wrong public key length")
            }
        }

        if self.bitmap.inner().len() != BITMAP_NUM_OF_BYTES {
            bail!(
                "MultiEd25519 signature has an invalid number of bitmap bytes {} expected {}",
                self.bitmap.inner().len(),
                BITMAP_NUM_OF_BYTES
            );
        }

        Ok(())
    }
}

impl TryFrom<&MultiEd25519Signature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &MultiEd25519Signature) -> Result<Self, Self::Error> {
        let ed25519_public_keys = value
            .public_keys
            .iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;
        let ed25519_signatures = value
            .signatures
            .iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;

        Ok(TransactionAuthenticator::multi_ed25519(
            MultiEd25519PublicKey::new(ed25519_public_keys, value.threshold)?,
            aptos_crypto::multi_ed25519::MultiEd25519Signature::new_with_signatures_and_bitmap(
                ed25519_signatures,
                value.bitmap.inner().try_into()?,
            ),
        ))
    }
}

impl TryFrom<&MultiEd25519Signature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &MultiEd25519Signature) -> Result<Self, Self::Error> {
        let ed25519_public_keys = value
            .public_keys
            .iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;
        let ed25519_signatures = value
            .signatures
            .iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;

        Ok(AccountAuthenticator::multi_ed25519(
            MultiEd25519PublicKey::new(ed25519_public_keys, value.threshold)?,
            aptos_crypto::multi_ed25519::MultiEd25519Signature::new_with_signatures_and_bitmap(
                ed25519_signatures,
                value.bitmap.inner().try_into()?,
            ),
        ))
    }
}

/// A single Secp256k1Ecdsa signature
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct Secp256k1EcdsaSignature {
    pub public_key: HexEncodedBytes,
    pub signature: HexEncodedBytes,
}

impl VerifyInput for Secp256k1EcdsaSignature {
    fn verify(&self) -> anyhow::Result<()> {
        let public_key_len = self.public_key.inner().len();
        let signature_len = self.signature.inner().len();
        if public_key_len != secp256k1_ecdsa::PUBLIC_KEY_LENGTH {
            bail!(
                "Secp256k1Ecdsa signature's public key is an invalid number of bytes, should be {} bytes but found {}",
                secp256k1_ecdsa::PUBLIC_KEY_LENGTH, public_key_len
            )
        } else if signature_len != secp256k1_ecdsa::SIGNATURE_LENGTH {
            bail!(
                "Secp256k1Ecdsa signature length is an invalid number of bytes, should be {} bytes but found {}",
                secp256k1_ecdsa::SIGNATURE_LENGTH, signature_len
            )
        } else {
            // TODO: Check if they match / parse correctly?
            Ok(())
        }
    }
}

/// A single WebAuthn signature
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct WebAuthnSignature {
    pub public_key: HexEncodedBytes,
    pub signature: HexEncodedBytes,
}

impl VerifyInput for WebAuthnSignature {
    fn verify(&self) -> anyhow::Result<()> {
        let public_key_len = self.public_key.inner().len();
        let signature_len = self.signature.inner().len();

        // Currently only takes Secp256r1Ecdsa. If other signature schemes are introduced, modify this to accommodate them
        if public_key_len != PUBLIC_KEY_LENGTH {
            bail!(
                "The public key provided is an invalid number of bytes, should be {} bytes but found {}. Note WebAuthn signatures only support Secp256r1Ecdsa at this time.",
                secp256r1_ecdsa::PUBLIC_KEY_LENGTH, public_key_len
            )
        } else if signature_len > MAX_WEBAUTHN_SIGNATURE_BYTES {
            bail!(
                "The WebAuthn signature length is greater than the maximum number of {} bytes: found {} bytes.",
                MAX_WEBAUTHN_SIGNATURE_BYTES, signature_len
            )
        } else {
            // TODO: Check if they match / parse correctly?
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct KeylessSignature {
    pub public_key: HexEncodedBytes,
    pub signature: HexEncodedBytes,
}

impl VerifyInput for KeylessSignature {
    fn verify(&self) -> anyhow::Result<()> {
        let public_key_len = self.public_key.inner().len();
        let signature_len = self.signature.inner().len();
        if public_key_len
            > std::cmp::max(
                keyless::KeylessPublicKey::MAX_LEN,
                keyless::FederatedKeylessPublicKey::MAX_LEN,
            )
        {
            bail!(
                "Keyless public key length is greater than the maximum number of {} bytes: found {} bytes",
                keyless::KeylessPublicKey::MAX_LEN, public_key_len
            )
        } else if signature_len > keyless::KeylessSignature::MAX_LEN {
            bail!(
                "Keyless signature length is greater than the maximum number of {} bytes: found {} bytes",
                keyless::KeylessSignature::MAX_LEN, signature_len
            )
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum Signature {
    Ed25519(Ed25519),
    Secp256k1Ecdsa(Secp256k1Ecdsa),
    WebAuthn(WebAuthn),
    Keyless(Keyless),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct Ed25519 {
    pub value: HexEncodedBytes,
}

impl Ed25519 {
    pub fn new(value: HexEncodedBytes) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct Secp256k1Ecdsa {
    pub value: HexEncodedBytes,
}

impl Secp256k1Ecdsa {
    pub fn new(value: HexEncodedBytes) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct Secp256r1Ecdsa {
    pub value: HexEncodedBytes,
}

impl Secp256r1Ecdsa {
    pub fn new(value: HexEncodedBytes) -> Self {
        Self { value }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct WebAuthn {
    pub value: HexEncodedBytes,
}

impl WebAuthn {
    pub fn new(value: HexEncodedBytes) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct Keyless {
    pub value: HexEncodedBytes,
}

impl Keyless {
    pub fn new(value: HexEncodedBytes) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct FederatedKeyless {
    pub value: HexEncodedBytes,
}

impl FederatedKeyless {
    pub fn new(value: HexEncodedBytes) -> Self {
        Self { value }
    }
}

impl TryFrom<&Signature> for AnySignature {
    type Error = anyhow::Error;

    fn try_from(signature: &Signature) -> Result<Self, Self::Error> {
        Ok(match signature {
            Signature::Ed25519(s) => AnySignature::ed25519(s.value.inner().try_into()?),
            Signature::Secp256k1Ecdsa(s) => {
                AnySignature::secp256k1_ecdsa(s.value.inner().try_into()?)
            },
            Signature::WebAuthn(s) => AnySignature::webauthn(s.value.inner().try_into()?),
            Signature::Keyless(s) => AnySignature::keyless(s.value.inner().try_into()?),
        })
    }
}

impl From<&AnySignature> for Signature {
    fn from(signature: &AnySignature) -> Self {
        match signature {
            AnySignature::Ed25519 { signature } => {
                Signature::Ed25519(Ed25519::new(signature.to_bytes().to_vec().into()))
            },
            AnySignature::Secp256k1Ecdsa { signature } => {
                Signature::Secp256k1Ecdsa(Secp256k1Ecdsa::new(signature.to_bytes().to_vec().into()))
            },
            AnySignature::WebAuthn { signature } => {
                Signature::WebAuthn(WebAuthn::new(signature.to_bytes().to_vec().into()))
            },
            AnySignature::Keyless { signature } => {
                Signature::Keyless(Keyless::new(signature.to_bytes().into()))
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum PublicKey {
    Ed25519(Ed25519),
    Secp256k1Ecdsa(Secp256k1Ecdsa),
    Secp256r1Ecdsa(Secp256r1Ecdsa),
    Keyless(Keyless),
    FederatedKeyless(FederatedKeyless),
}

impl TryFrom<&PublicKey> for AnyPublicKey {
    type Error = anyhow::Error;

    fn try_from(public_key: &PublicKey) -> Result<Self, Self::Error> {
        Ok(match public_key {
            PublicKey::Ed25519(p) => AnyPublicKey::ed25519(p.value.inner().try_into()?),
            PublicKey::Secp256k1Ecdsa(p) => {
                AnyPublicKey::secp256k1_ecdsa(p.value.inner().try_into()?)
            },
            PublicKey::Secp256r1Ecdsa(p) => {
                AnyPublicKey::secp256r1_ecdsa(p.value.inner().try_into()?)
            },
            PublicKey::Keyless(p) => AnyPublicKey::keyless(p.value.inner().try_into()?),
            PublicKey::FederatedKeyless(p) => {
                AnyPublicKey::federated_keyless(p.value.inner().try_into()?)
            },
        })
    }
}

impl From<&AnyPublicKey> for PublicKey {
    fn from(key: &AnyPublicKey) -> Self {
        match key {
            AnyPublicKey::Ed25519 { public_key } => {
                PublicKey::Ed25519(Ed25519::new(public_key.to_bytes().to_vec().into()))
            },
            AnyPublicKey::Secp256k1Ecdsa { public_key } => PublicKey::Secp256k1Ecdsa(
                Secp256k1Ecdsa::new(public_key.to_bytes().to_vec().into()),
            ),
            AnyPublicKey::Secp256r1Ecdsa { public_key } => PublicKey::Secp256r1Ecdsa(
                Secp256r1Ecdsa::new(public_key.to_bytes().to_vec().into()),
            ),
            AnyPublicKey::Keyless { public_key } => {
                PublicKey::Keyless(Keyless::new(public_key.to_bytes().into()))
            },
            AnyPublicKey::FederatedKeyless { public_key } => {
                PublicKey::FederatedKeyless(FederatedKeyless::new(public_key.to_bytes().into()))
            },
        }
    }
}

/// A single key signature
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct SingleKeySignature {
    pub public_key: PublicKey,
    pub signature: Signature,
}

impl VerifyInput for SingleKeySignature {
    fn verify(&self) -> anyhow::Result<()> {
        match (&self.public_key, &self.signature) {
            (PublicKey::Ed25519(p), Signature::Ed25519(s)) => Ed25519Signature {
                public_key: p.value.clone(),
                signature: s.value.clone(),
            }
            .verify(),
            (PublicKey::Secp256k1Ecdsa(p), Signature::Secp256k1Ecdsa(s)) => {
                Secp256k1EcdsaSignature {
                    public_key: p.value.clone(),
                    signature: s.value.clone(),
                }
                .verify()
            },
            (PublicKey::Secp256r1Ecdsa(p), Signature::WebAuthn(s)) => WebAuthnSignature {
                public_key: p.value.clone(),
                signature: s.value.clone(),
            }
            .verify(),
            (PublicKey::Keyless(p), Signature::Keyless(s)) => KeylessSignature {
                public_key: p.value.clone(),
                signature: s.value.clone(),
            }
            .verify(),
            (PublicKey::FederatedKeyless(p), Signature::Keyless(s)) => KeylessSignature {
                public_key: p.value.clone(),
                signature: s.value.clone(),
            }
            .verify(),
            _ => bail!("Invalid public key, signature match."),
        }
    }
}

impl TryFrom<&SingleKeySignature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(signature: &SingleKeySignature) -> Result<Self, Self::Error> {
        let account_auth = signature.try_into()?;
        Ok(TransactionAuthenticator::single_sender(account_auth))
    }
}

impl TryFrom<&SingleKeySignature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &SingleKeySignature) -> Result<Self, Self::Error> {
        let key =
            match value.public_key {
                PublicKey::Ed25519(ref p) => {
                    let key =
                        p.value.inner().try_into().context(
                            "Failed to parse given public_key bytes as Ed25519PublicKey",
                        )?;
                    AnyPublicKey::ed25519(key)
                },
                PublicKey::Secp256k1Ecdsa(ref p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as Secp256k1EcdsaPublicKey",
                    )?;
                    AnyPublicKey::secp256k1_ecdsa(key)
                },
                PublicKey::Secp256r1Ecdsa(ref p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as Secp256r1EcdsaPublicKey",
                    )?;
                    AnyPublicKey::secp256r1_ecdsa(key)
                },
                PublicKey::Keyless(ref p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as AnyPublicKey::Keyless",
                    )?;
                    AnyPublicKey::keyless(key)
                },
                PublicKey::FederatedKeyless(ref p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as AnyPublicKey::FederatedKeyless",
                    )?;
                    AnyPublicKey::keyless(key)
                },
            };

        let signature = match value.signature {
            Signature::Ed25519(ref s) => {
                let signature = s
                    .value
                    .inner()
                    .try_into()
                    .context("Failed to parse given signature bytes as Ed25519Signature")?;
                AnySignature::ed25519(signature)
            },
            Signature::Secp256k1Ecdsa(ref s) => {
                let signature =
                    s.value.inner().try_into().context(
                        "Failed to parse given signature bytes as Secp256k1EcdsaSignature",
                    )?;
                AnySignature::secp256k1_ecdsa(signature)
            },
            Signature::WebAuthn(ref s) => {
                let signature = s
                    .value
                    .inner()
                    .try_into()
                    .context( "Failed to parse given signature bytes as PartialAuthenticatorAssertionResponse")?;
                AnySignature::webauthn(signature)
            },
            Signature::Keyless(ref s) => {
                let signature =
                    s.value.inner().try_into().context(
                        "Failed to parse given signature bytes as AnySignature::Keyless",
                    )?;
                AnySignature::keyless(signature)
            },
        };

        let auth = SingleKeyAuthenticator::new(key, signature);
        Ok(AccountAuthenticator::single_key(auth))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct IndexedSignature {
    pub index: u8,
    pub signature: Signature,
}

/// A multi key signature
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MultiKeySignature {
    pub public_keys: Vec<PublicKey>,
    pub signatures: Vec<IndexedSignature>,
    pub signatures_required: u8,
}

impl VerifyInput for MultiKeySignature {
    fn verify(&self) -> anyhow::Result<()> {
        if self.public_keys.is_empty() {
            bail!("MultiKey signature has no public keys")
        } else if self.signatures.is_empty() {
            bail!("MultiKey signature has no signatures")
        } else if self.public_keys.len() > MAX_NUM_OF_KEYS {
            bail!(
                "MultiKey signature has over the maximum number of public keys {}",
                MAX_NUM_OF_KEYS
            )
        } else if self.signatures.len() > MAX_NUM_OF_SIGS {
            bail!(
                "MultiKey signature has over the maximum number of signatures {}",
                MAX_NUM_OF_SIGS
            )
        } else if self.signatures.len() != self.signatures_required as usize {
            bail!("MultiKey signature does not the number of signatures required")
        } else if self.signatures_required == 0 {
            bail!("MultiKey signature threshold must be greater than 0")
        } else if self.signatures_required > MAX_NUM_OF_SIGS as u8 {
            bail!("MultiKey signature threshold is greater than the maximum number of signatures")
        }
        let _: AccountAuthenticator = self.try_into()?;
        Ok(())
    }
}

impl TryFrom<&MultiKeySignature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(signature: &MultiKeySignature) -> Result<Self, Self::Error> {
        let account_auth = signature.try_into()?;
        Ok(TransactionAuthenticator::single_sender(account_auth))
    }
}

impl TryFrom<&MultiKeySignature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &MultiKeySignature) -> Result<Self, Self::Error> {
        let mut public_keys = vec![];
        for public_key in value.public_keys.iter() {
            let key = match public_key {
                PublicKey::Ed25519(p) => {
                    let key =
                        p.value.inner().try_into().context(
                            "Failed to parse given public_key bytes as Ed25519PublicKey",
                        )?;
                    AnyPublicKey::ed25519(key)
                },
                PublicKey::Secp256k1Ecdsa(p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as Secp256k1EcdsaPublicKey",
                    )?;
                    AnyPublicKey::secp256k1_ecdsa(key)
                },
                PublicKey::Secp256r1Ecdsa(p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as Secp256r1EcdsaPublicKey",
                    )?;
                    AnyPublicKey::secp256r1_ecdsa(key)
                },
                PublicKey::Keyless(p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as AnyPublicKey::Keyless",
                    )?;
                    AnyPublicKey::keyless(key)
                },
                PublicKey::FederatedKeyless(p) => {
                    let key = p.value.inner().try_into().context(
                        "Failed to parse given public_key bytes as AnyPublicKey::FederatedKeyless",
                    )?;
                    AnyPublicKey::federated_keyless(key)
                },
            };
            public_keys.push(key);
        }

        let mut signatures = vec![];
        for indexed_signature in value.signatures.iter() {
            let signature =
                match &indexed_signature.signature {
                    Signature::Ed25519(s) => {
                        let signature = s.value.inner().try_into().context(
                            "Failed to parse given public_key bytes as Ed25519Signature",
                        )?;
                        AnySignature::ed25519(signature)
                    },
                    Signature::Secp256k1Ecdsa(s) => {
                        let signature = s.value.inner().try_into().context(
                            "Failed to parse given signature as Secp256k1EcdsaSignature",
                        )?;
                        AnySignature::secp256k1_ecdsa(signature)
                    },
                    Signature::WebAuthn(s) => {
                        let paar = s.value.inner().try_into().context(
                        "Failed to parse given signature as PartialAuthenticatorAssertionResponse",
                    )?;
                        AnySignature::webauthn(paar)
                    },
                    Signature::Keyless(s) => {
                        let signature =
                            s.value.inner().try_into().context(
                                "Failed to parse given signature as AnySignature::Keyless",
                            )?;
                        AnySignature::keyless(signature)
                    },
                };
            signatures.push((indexed_signature.index, signature));
        }

        let multi_key = MultiKey::new(public_keys, value.signatures_required)?;
        let auth = MultiKeyAuthenticator::new(multi_key, signatures)?;
        Ok(AccountAuthenticator::multi_key(auth))
    }
}

/// A placeholder to represent the absence of account signature
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct NoAccountSignature;

impl VerifyInput for NoAccountSignature {
    fn verify(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct AbstractSignature {
    pub function_info: String,
    pub auth_data: HexEncodedBytes,
}

impl VerifyInput for AbstractSignature {
    fn verify(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl TryFrom<&NoAccountSignature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(signature: &NoAccountSignature) -> Result<Self, Self::Error> {
        let account_auth = signature.try_into()?;
        Ok(TransactionAuthenticator::single_sender(account_auth))
    }
}

impl TryFrom<&NoAccountSignature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(_value: &NoAccountSignature) -> Result<Self, Self::Error> {
        Ok(AccountAuthenticator::NoAccountAuthenticator)
    }
}

impl TryFrom<&AbstractSignature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &AbstractSignature) -> Result<Self, Self::Error> {
        Ok(AccountAuthenticator::Abstract {
            authenticator: AbstractAuthenticator::new(
                FunctionInfo::from_str(&value.function_info)?,
                bcs::from_bytes(value.auth_data.inner())?,
            ),
        })
    }
}

/// Account signature scheme
///
/// The account signature scheme allows you to have two types of accounts:
///
///   1. A single Ed25519 key account, one private key
///   2. A k-of-n multi-Ed25519 key account, multiple private keys, such that k-of-n must sign a transaction.
///   3. A single Secp256k1Ecdsa key account, one private key
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Union)]
#[serde(tag = "type", rename_all = "snake_case")]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum AccountSignature {
    Ed25519Signature(Ed25519Signature),
    MultiEd25519Signature(MultiEd25519Signature),
    SingleKeySignature(SingleKeySignature),
    MultiKeySignature(MultiKeySignature),
    NoAccountSignature(NoAccountSignature),
    AbstractSignature(AbstractSignature),
}

impl VerifyInput for AccountSignature {
    fn verify(&self) -> anyhow::Result<()> {
        match self {
            AccountSignature::Ed25519Signature(inner) => inner.verify(),
            AccountSignature::MultiEd25519Signature(inner) => inner.verify(),
            AccountSignature::SingleKeySignature(inner) => inner.verify(),
            AccountSignature::MultiKeySignature(inner) => inner.verify(),
            AccountSignature::NoAccountSignature(inner) => inner.verify(),
            AccountSignature::AbstractSignature(inner) => inner.verify(),
        }
    }
}

impl TryFrom<&AccountSignature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(sig: &AccountSignature) -> anyhow::Result<Self> {
        Ok(match sig {
            AccountSignature::Ed25519Signature(s) => s.try_into()?,
            AccountSignature::MultiEd25519Signature(s) => s.try_into()?,
            AccountSignature::SingleKeySignature(s) => s.try_into()?,
            AccountSignature::MultiKeySignature(s) => s.try_into()?,
            AccountSignature::NoAccountSignature(s) => s.try_into()?,
            AccountSignature::AbstractSignature(s) => s.try_into()?,
        })
    }
}

/// Multi agent signature for multi agent transactions
///
/// This allows you to have transactions across multiple accounts
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MultiAgentSignature {
    pub sender: AccountSignature,
    /// The other involved parties' addresses
    pub secondary_signer_addresses: Vec<Address>,
    /// The associated signatures, in the same order as the secondary addresses
    pub secondary_signers: Vec<AccountSignature>,
}

impl VerifyInput for MultiAgentSignature {
    fn verify(&self) -> anyhow::Result<()> {
        self.sender.verify()?;

        if self.secondary_signer_addresses.is_empty() {
            bail!("MultiAgent signature has no secondary signer addresses")
        } else if self.secondary_signers.is_empty() {
            bail!("MultiAgent signature has no secondary signatures")
        } else if self.secondary_signers.len() != self.secondary_signer_addresses.len() {
            bail!("MultiAgent signatures don't match addresses length")
        }

        for signer in self.secondary_signers.iter() {
            signer.verify()?;
        }
        Ok(())
    }
}

impl TryFrom<&MultiAgentSignature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &MultiAgentSignature) -> Result<Self, Self::Error> {
        Ok(TransactionAuthenticator::multi_agent(
            (&value.sender).try_into()?,
            value
                .secondary_signer_addresses
                .iter()
                .map(|a| a.into())
                .collect(),
            value
                .secondary_signers
                .iter()
                .map(|s| s.try_into())
                .collect::<anyhow::Result<_>>()?,
        ))
    }
}

impl From<(&Ed25519PublicKey, &ed25519::Ed25519Signature)> for Ed25519Signature {
    fn from((pk, sig): (&Ed25519PublicKey, &ed25519::Ed25519Signature)) -> Self {
        Self {
            public_key: pk.to_bytes().to_vec().into(),
            signature: sig.to_bytes().to_vec().into(),
        }
    }
}

impl
    From<(
        &MultiEd25519PublicKey,
        &multi_ed25519::MultiEd25519Signature,
    )> for MultiEd25519Signature
{
    fn from(
        (pk, sig): (
            &MultiEd25519PublicKey,
            &multi_ed25519::MultiEd25519Signature,
        ),
    ) -> Self {
        Self {
            public_keys: pk
                .public_keys()
                .iter()
                .map(|k| k.to_bytes().to_vec().into())
                .collect(),
            signatures: sig
                .signatures()
                .iter()
                .map(|s| s.to_bytes().to_vec().into())
                .collect(),
            threshold: *pk.threshold(),
            bitmap: sig.bitmap().to_vec().into(),
        }
    }
}

impl From<(&secp256k1_ecdsa::PublicKey, &secp256k1_ecdsa::Signature)> for Secp256k1EcdsaSignature {
    fn from((pk, sig): (&secp256k1_ecdsa::PublicKey, &secp256k1_ecdsa::Signature)) -> Self {
        Self {
            public_key: pk.to_bytes().to_vec().into(),
            signature: sig.to_bytes().to_vec().into(),
        }
    }
}

impl
    From<(
        &secp256r1_ecdsa::PublicKey,
        &PartialAuthenticatorAssertionResponse,
    )> for Secp256k1EcdsaSignature
{
    fn from(
        (pk, sig): (
            &secp256r1_ecdsa::PublicKey,
            &PartialAuthenticatorAssertionResponse,
        ),
    ) -> Self {
        Self {
            public_key: pk.to_bytes().to_vec().into(),
            signature: sig.to_bytes().to_vec().into(),
        }
    }
}

impl From<&AccountAuthenticator> for AccountSignature {
    fn from(auth: &AccountAuthenticator) -> Self {
        use AccountAuthenticator::*;
        match auth {
            Ed25519 {
                public_key,
                signature,
            } => Self::Ed25519Signature((public_key, signature).into()),
            MultiEd25519 {
                public_key,
                signature,
            } => Self::MultiEd25519Signature((public_key, signature).into()),
            SingleKey { authenticator } => Self::SingleKeySignature(SingleKeySignature {
                public_key: authenticator.public_key().into(),
                signature: authenticator.signature().into(),
            }),
            MultiKey { authenticator } => {
                let public_keys = authenticator.public_keys();
                let signatures = authenticator.signatures();

                Self::MultiKeySignature(MultiKeySignature {
                    public_keys: public_keys
                        .public_keys()
                        .iter()
                        .map(|pk| pk.into())
                        .collect(),
                    signatures: signatures
                        .into_iter()
                        .map(|(index, signature)| IndexedSignature {
                            index,
                            signature: signature.into(),
                        })
                        .collect(),
                    signatures_required: public_keys.signatures_required(),
                })
            },
            NoAccountAuthenticator => AccountSignature::NoAccountSignature(NoAccountSignature),
            Abstract { authenticator } => Self::AbstractSignature(AbstractSignature {
                function_info: authenticator.function_info().to_string(),
                auth_data: to_bytes(authenticator.auth_data())
                    .expect("bcs serialization cannot fail")
                    .into(),
            }),
        }
    }
}

impl
    From<(
        &AccountAuthenticator,
        &Vec<AccountAddress>,
        &Vec<AccountAuthenticator>,
    )> for MultiAgentSignature
{
    fn from(
        (sender, addresses, signers): (
            &AccountAuthenticator,
            &Vec<AccountAddress>,
            &Vec<AccountAuthenticator>,
        ),
    ) -> Self {
        Self {
            sender: sender.into(),
            secondary_signer_addresses: addresses.iter().map(|address| (*address).into()).collect(),
            secondary_signers: signers.iter().map(|s| s.into()).collect(),
        }
    }
}

/// Fee payer signature for fee payer transactions
///
/// This allows you to have transactions across multiple accounts and with a fee payer
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct FeePayerSignature {
    pub sender: AccountSignature,
    /// The other involved parties' addresses
    pub secondary_signer_addresses: Vec<Address>,
    /// The associated signatures, in the same order as the secondary addresses
    pub secondary_signers: Vec<AccountSignature>,
    /// The address of the paying party
    pub fee_payer_address: Address,
    /// The signature of the fee payer
    pub fee_payer_signer: AccountSignature,
}

impl VerifyInput for FeePayerSignature {
    fn verify(&self) -> anyhow::Result<()> {
        self.sender.verify()?;

        for signer in self.secondary_signers.iter() {
            signer.verify()?;
        }
        self.fee_payer_signer.verify()?;
        Ok(())
    }
}

impl TryFrom<&FeePayerSignature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: &FeePayerSignature) -> Result<Self, Self::Error> {
        Ok(TransactionAuthenticator::fee_payer(
            (&value.sender).try_into()?,
            value
                .secondary_signer_addresses
                .iter()
                .map(|a| a.into())
                .collect(),
            value
                .secondary_signers
                .iter()
                .map(|s| s.try_into())
                .collect::<anyhow::Result<_>>()?,
            value.fee_payer_address.into(),
            (&value.fee_payer_signer).try_into()?,
        ))
    }
}

impl
    From<(
        &AccountAuthenticator,
        &Vec<AccountAddress>,
        &Vec<AccountAuthenticator>,
        &AccountAddress,
        &AccountAuthenticator,
    )> for FeePayerSignature
{
    fn from(
        (sender, addresses, signers, fee_payer_address, fee_payer_signer): (
            &AccountAuthenticator,
            &Vec<AccountAddress>,
            &Vec<AccountAuthenticator>,
            &AccountAddress,
            &AccountAuthenticator,
        ),
    ) -> Self {
        Self {
            sender: sender.into(),
            secondary_signer_addresses: addresses.iter().map(|address| (*address).into()).collect(),
            secondary_signers: signers.iter().map(|s| s.into()).collect(),
            fee_payer_address: (*fee_payer_address).into(),
            fee_payer_signer: fee_payer_signer.into(),
        }
    }
}

impl From<TransactionAuthenticator> for TransactionSignature {
    fn from(auth: TransactionAuthenticator) -> Self {
        use TransactionAuthenticator::*;
        match &auth {
            Ed25519 {
                public_key,
                signature,
            } => Self::Ed25519Signature((public_key, signature).into()),
            MultiEd25519 {
                public_key,
                signature,
            } => Self::MultiEd25519Signature((public_key, signature).into()),
            MultiAgent {
                sender,
                secondary_signer_addresses,
                secondary_signers,
            } => Self::MultiAgentSignature(
                (sender, secondary_signer_addresses, secondary_signers).into(),
            ),
            FeePayer {
                sender,
                secondary_signer_addresses,
                secondary_signers,
                fee_payer_address,
                fee_payer_signer,
            } => Self::FeePayerSignature(
                (
                    sender,
                    secondary_signer_addresses,
                    secondary_signers,
                    fee_payer_address,
                    fee_payer_signer,
                )
                    .into(),
            ),
            SingleSender { sender } => Self::SingleSender(sender.into()),
            NoneForFuzz => Self::NoAccountSignature(NoAccountSignature),
        }
    }
}

/// A transaction identifier
///
/// There are 2 types transaction ids from HTTP request inputs:
/// 1. Transaction hash: hex-encoded string, e.g. "0x374eda71dce727c6cd2dd4a4fd47bfb85c16be2e3e95ab0df4948f39e1af9981"
/// 2. Transaction version: u64 number string (as we encode u64 into string in JSON), e.g. "122"
#[derive(Clone, Debug, Union)]
#[oai(one_of, discriminator_name = "type", rename_all = "snake_case")]
pub enum TransactionId {
    Hash(HashValue),
    Version(U64),
}

impl FromStr for TransactionId {
    type Err = anyhow::Error;

    fn from_str(hash_or_version: &str) -> Result<Self, anyhow::Error> {
        let id = match hash_or_version.parse::<u64>() {
            Ok(version) => TransactionId::Version(U64::from(version)),
            Err(_) => TransactionId::Hash(hash_or_version.parse()?),
        };
        Ok(id)
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Hash(h) => write!(f, "hash({})", h),
            Self::Version(v) => write!(f, "version({})", v),
        }
    }
}

/// A hex encoded BCS encoded transaction from the EncodeSubmission API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct TransactionSigningMessage {
    pub message: HexEncodedBytes,
}

impl TransactionSigningMessage {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            message: bytes.into(),
        }
    }
}

/// Struct holding the outputs of the estimate gas API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct GasEstimationBcs {
    /// The current estimate for the gas unit price
    pub gas_estimate: u64,
}

/// Struct holding the outputs of the estimate gas API
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct GasEstimation {
    /// The deprioritized estimate for the gas unit price
    pub deprioritized_gas_estimate: Option<u64>,
    /// The current estimate for the gas unit price
    pub gas_estimate: u64,
    /// The prioritized estimate for the gas unit price
    pub prioritized_gas_estimate: Option<u64>,
}
