// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Address, EventKey, HashValue, HexEncodedBytes, MoveModuleBytecode, MoveModuleId, MoveResource,
    MoveResourceType, MoveScriptBytecode, MoveType, MoveValue, U64,
};

use diem_crypto::{
    ed25519::{self, Ed25519PublicKey},
    multi_ed25519::{self, MultiEd25519PublicKey},
    validatable::Validatable,
};
use diem_types::{
    account_address::AccountAddress,
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    transaction::{
        authenticator::{AccountAuthenticator, TransactionAuthenticator},
        default_protocol::TransactionWithProof,
        Script, SignedTransaction, TransactionInfoTrait,
    },
};
use move_core_types::identifier::Identifier;

use serde::{Deserialize, Serialize};
use std::{
    boxed::Box,
    convert::{From, Into, TryFrom, TryInto},
    fmt,
    str::FromStr,
};

#[derive(Clone, Debug)]
pub enum TransactionData<T: TransactionInfoTrait> {
    OnChain(TransactionOnChainData<T>),
    Pending(Box<SignedTransaction>),
}

impl<T: TransactionInfoTrait> From<TransactionOnChainData<T>> for TransactionData<T> {
    fn from(txn: TransactionOnChainData<T>) -> Self {
        Self::OnChain(txn)
    }
}

impl<T: TransactionInfoTrait> From<SignedTransaction> for TransactionData<T> {
    fn from(txn: SignedTransaction) -> Self {
        Self::Pending(Box::new(txn))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionOnChainData<T: TransactionInfoTrait> {
    pub version: u64,
    pub transaction: diem_types::transaction::Transaction,
    pub info: T,
    pub events: Vec<ContractEvent>,
}

impl From<TransactionWithProof>
    for TransactionOnChainData<diem_types::transaction::TransactionInfo>
{
    fn from(txn: TransactionWithProof) -> Self {
        Self {
            version: txn.version,
            transaction: txn.transaction,
            info: txn.proof.transaction_info,
            events: txn.events.unwrap_or_default(),
        }
    }
}

impl<T: TransactionInfoTrait>
    From<(
        u64,
        diem_types::transaction::Transaction,
        T,
        Vec<ContractEvent>,
    )> for TransactionOnChainData<T>
{
    fn from(
        (version, transaction, info, events): (
            u64,
            diem_types::transaction::Transaction,
            T,
            Vec<ContractEvent>,
        ),
    ) -> Self {
        Self {
            version,
            transaction,
            info,
            events,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transaction {
    PendingTransaction(PendingTransaction),
    UserTransaction(Box<UserTransaction>),
    GenesisTransaction(GenesisTransaction),
    BlockMetadataTransaction(BlockMetadataTransaction),
}

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
    )> for Transaction
{
    fn from(
        (txn, info, payload, events): (
            &SignedTransaction,
            TransactionInfo,
            TransactionPayload,
            Vec<Event>,
        ),
    ) -> Self {
        Transaction::UserTransaction(Box::new(UserTransaction {
            info,
            request: (txn, payload).into(),
            events,
        }))
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

impl From<(&BlockMetadata, TransactionInfo)> for Transaction {
    fn from((txn, info): (&BlockMetadata, TransactionInfo)) -> Self {
        Transaction::BlockMetadataTransaction(BlockMetadataTransaction {
            info,
            id: txn.id().into(),
            round: txn.round().into(),
            previous_block_votes: txn
                .previous_block_votes()
                .clone()
                .iter()
                .map(|a| (*a).into())
                .collect(),
            proposer: txn.proposer().into(),
            timestamp: txn.timestamp_usec().into(),
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
            gas_currency_code: txn.gas_currency_code().to_owned(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs().into(),
            signature: Some(txn.authenticator().into()),
            payload,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransactionInfo {
    pub version: U64,
    pub hash: HashValue,
    pub state_root_hash: HashValue,
    pub event_root_hash: HashValue,
    pub gas_used: U64,
    pub success: bool,
    pub vm_status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PendingTransaction {
    pub hash: HashValue,
    #[serde(flatten)]
    pub request: UserTransactionRequest,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UserTransaction {
    #[serde(flatten)]
    pub info: TransactionInfo,
    #[serde(flatten)]
    pub request: UserTransactionRequest,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserTransactionRequest {
    pub sender: Address,
    pub sequence_number: U64,
    pub max_gas_amount: U64,
    pub gas_unit_price: U64,
    pub gas_currency_code: String,
    pub expiration_timestamp_secs: U64,
    pub payload: TransactionPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<TransactionSignature>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GenesisTransaction {
    #[serde(flatten)]
    pub info: TransactionInfo,
    pub payload: GenesisPayload,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BlockMetadataTransaction {
    #[serde(flatten)]
    pub info: TransactionInfo,
    pub id: HashValue,
    pub round: U64,
    pub previous_block_votes: Vec<Address>,
    pub proposer: Address,
    pub timestamp: U64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub key: EventKey,
    pub sequence_number: U64,
    #[serde(rename = "type")]
    pub typ: MoveType,
    pub data: serde_json::Value,
}

impl From<(&ContractEvent, serde_json::Value)> for Event {
    fn from((event, data): (&ContractEvent, serde_json::Value)) -> Self {
        match event {
            ContractEvent::V0(v0) => Self {
                key: (*v0.key()).into(),
                sequence_number: v0.sequence_number().into(),
                typ: v0.type_tag().clone().into(),
                data,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenesisPayload {
    WriteSetPayload(WriteSetPayload),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransactionPayload {
    ScriptFunctionPayload(ScriptFunctionPayload),
    ScriptPayload(ScriptPayload),
    ModulePayload(MoveModuleBytecode),
    WriteSetPayload(WriteSetPayload),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptFunctionPayload {
    pub module: MoveModuleId,
    pub function: Identifier,
    pub type_arguments: Vec<MoveType>,
    pub arguments: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptPayload {
    pub code: MoveScriptBytecode,
    pub type_arguments: Vec<MoveType>,
    pub arguments: Vec<serde_json::Value>,
}

impl TryFrom<Script> for ScriptPayload {
    type Error = anyhow::Error;

    fn try_from(script: Script) -> anyhow::Result<Self> {
        let (code, ty_args, args) = script.into_inner();
        Ok(Self {
            code: MoveScriptBytecode::new(code).try_parse_abi(),
            type_arguments: ty_args.into_iter().map(|arg| arg.into()).collect(),
            arguments: args
                .into_iter()
                .map(|arg| MoveValue::from(arg).json())
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WriteSetPayload {
    pub write_set: WriteSet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WriteSet {
    ScriptWriteSet(ScriptWriteSet),
    DirectWriteSet(DirectWriteSet),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptWriteSet {
    pub execute_as: Address,
    pub script: ScriptPayload,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectWriteSet {
    pub changes: Vec<WriteSetChange>,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WriteSetChange {
    DeleteModule {
        address: Address,
        module: MoveModuleId,
    },
    DeleteResource {
        address: Address,
        resource: MoveResourceType,
    },
    WriteModule {
        address: Address,
        data: MoveModuleBytecode,
    },
    WriteResource {
        address: Address,
        data: MoveResource,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransactionSignature {
    Ed25519Signature(Ed25519Signature),
    MultiEd25519Signature(MultiEd25519Signature),
    MultiAgentSignature(MultiAgentSignature),
}

impl TryFrom<TransactionSignature> for TransactionAuthenticator {
    type Error = anyhow::Error;
    fn try_from(ts: TransactionSignature) -> anyhow::Result<Self> {
        Ok(match ts {
            TransactionSignature::Ed25519Signature(sig) => sig.try_into()?,
            TransactionSignature::MultiEd25519Signature(sig) => sig.try_into()?,
            TransactionSignature::MultiAgentSignature(sig) => sig.try_into()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ed25519Signature {
    public_key: HexEncodedBytes,
    signature: HexEncodedBytes,
}

impl TryFrom<Ed25519Signature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: Ed25519Signature) -> Result<Self, Self::Error> {
        let Ed25519Signature {
            public_key,
            signature,
        } = value;
        Ok(TransactionAuthenticator::ed25519(
            public_key.inner().try_into()?,
            signature.inner().try_into()?,
        ))
    }
}

impl TryFrom<Ed25519Signature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: Ed25519Signature) -> Result<Self, Self::Error> {
        let Ed25519Signature {
            public_key,
            signature,
        } = value;
        Ok(AccountAuthenticator::ed25519(
            public_key.inner().try_into()?,
            signature.inner().try_into()?,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MultiEd25519Signature {
    public_keys: Vec<HexEncodedBytes>,
    signatures: Vec<HexEncodedBytes>,
    threshold: u8,
    bitmap: HexEncodedBytes,
}

impl TryFrom<MultiEd25519Signature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: MultiEd25519Signature) -> Result<Self, Self::Error> {
        let MultiEd25519Signature {
            public_keys,
            signatures,
            threshold,
            bitmap,
        } = value;

        let ed25519_public_keys = public_keys
            .into_iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;
        let ed25519_signatures = signatures
            .into_iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;

        Ok(TransactionAuthenticator::multi_ed25519(
            MultiEd25519PublicKey::new(ed25519_public_keys, threshold)?,
            diem_crypto::multi_ed25519::MultiEd25519Signature::new_with_signatures_and_bitmap(
                ed25519_signatures,
                bitmap.inner().try_into()?,
            ),
        ))
    }
}

impl TryFrom<MultiEd25519Signature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: MultiEd25519Signature) -> Result<Self, Self::Error> {
        let MultiEd25519Signature {
            public_keys,
            signatures,
            threshold,
            bitmap,
        } = value;

        let ed25519_public_keys = public_keys
            .into_iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;
        let ed25519_signatures = signatures
            .into_iter()
            .map(|s| Ok(s.inner().try_into()?))
            .collect::<anyhow::Result<_>>()?;

        Ok(AccountAuthenticator::multi_ed25519(
            MultiEd25519PublicKey::new(ed25519_public_keys, threshold)?,
            diem_crypto::multi_ed25519::MultiEd25519Signature::new_with_signatures_and_bitmap(
                ed25519_signatures,
                bitmap.inner().try_into()?,
            ),
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AccountSignature {
    Ed25519Signature(Ed25519Signature),
    MultiEd25519Signature(MultiEd25519Signature),
}

impl TryFrom<AccountSignature> for AccountAuthenticator {
    type Error = anyhow::Error;

    fn try_from(sig: AccountSignature) -> anyhow::Result<Self> {
        Ok(match sig {
            AccountSignature::Ed25519Signature(s) => s.try_into()?,
            AccountSignature::MultiEd25519Signature(s) => s.try_into()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MultiAgentSignature {
    sender: AccountSignature,
    secondary_signer_addresses: Vec<Address>,
    secondary_signers: Vec<AccountSignature>,
}

impl TryFrom<MultiAgentSignature> for TransactionAuthenticator {
    type Error = anyhow::Error;

    fn try_from(value: MultiAgentSignature) -> Result<Self, Self::Error> {
        let MultiAgentSignature {
            sender,
            secondary_signer_addresses,
            secondary_signers,
        } = value;
        Ok(TransactionAuthenticator::multi_agent(
            sender.try_into()?,
            secondary_signer_addresses
                .into_iter()
                .map(|a| a.into())
                .collect(),
            secondary_signers
                .into_iter()
                .map(|s| s.try_into())
                .collect::<anyhow::Result<_>>()?,
        ))
    }
}

impl From<(&Validatable<Ed25519PublicKey>, &ed25519::Ed25519Signature)> for Ed25519Signature {
    fn from((pk, sig): (&Validatable<Ed25519PublicKey>, &ed25519::Ed25519Signature)) -> Self {
        Self {
            public_key: pk.unvalidated().to_bytes().to_vec().into(),
            signature: sig.to_bytes().to_vec().into(),
        }
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
        }
    }
}

/// There are 2 types transaction ids from HTTP request inputs:
/// 1. Transaction hash: hex-encoded string, e.g. "0x374eda71dce727c6cd2dd4a4fd47bfb85c16be2e3e95ab0df4948f39e1af9981"
/// 2. Transaction version: u64 number string (as we encode u64 into string in JSON), e.g. "122"
#[derive(Clone, Debug)]
pub enum TransactionId {
    Hash(HashValue),
    Version(u64),
}

impl FromStr for TransactionId {
    type Err = anyhow::Error;

    fn from_str(hash_or_version: &str) -> Result<Self, anyhow::Error> {
        let id = match hash_or_version.parse::<u64>() {
            Ok(version) => TransactionId::Version(version),
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
