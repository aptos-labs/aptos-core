// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Address, EventKey, HashValue, HexEncodedBytes, MoveModuleId, MoveType, MoveValue, U64,
};

use diem_crypto::hash::CryptoHash;
use diem_types::{
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    transaction::{Script, SignedTransaction, TransactionInfoTrait, WriteSetPayload},
};
use move_core_types::identifier::Identifier;
use resource_viewer::AnnotatedMoveValue;

use serde::Serialize;
use std::{boxed::Box, convert::From};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transaction {
    PendingTransaction(PendingTransaction),
    UserTransaction(Box<UserTransaction>),
    GenesisTransaction(GenesisTransaction),
    BlockMetadataTransaction(BlockMetadataTransaction),
}

impl Transaction {
    pub fn hash(txn: SignedTransaction) -> HashValue {
        diem_types::transaction::Transaction::UserTransaction(txn)
            .hash()
            .into()
    }
}

impl From<SignedTransaction> for Transaction {
    fn from(txn: SignedTransaction) -> Self {
        Transaction::PendingTransaction(PendingTransaction {
            sender: txn.sender().into(),
            sequence_number: txn.sequence_number().into(),
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            gas_currency_code: txn.gas_currency_code().to_owned(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs().into(),
            hash: Transaction::hash(txn),
        })
    }
}

impl<T: TransactionInfoTrait> From<(u64, &SignedTransaction, &T, Vec<Event>, TransactionPayload)>
    for Transaction
{
    fn from(
        (version, txn, info, events, payload): (
            u64,
            &SignedTransaction,
            &T,
            Vec<Event>,
            TransactionPayload,
        ),
    ) -> Self {
        Transaction::UserTransaction(Box::new(UserTransaction {
            version: version.into(),
            hash: info.transaction_hash().into(),
            state_root_hash: info.state_root_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),

            sender: txn.sender().into(),
            sequence_number: txn.sequence_number().into(),
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            gas_currency_code: txn.gas_currency_code().to_owned(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs().into(),
            events,
            payload,
        }))
    }
}

impl<T: TransactionInfoTrait> From<(u64, &WriteSetPayload, &T, Vec<Event>)> for Transaction {
    fn from((version, txn, info, events): (u64, &WriteSetPayload, &T, Vec<Event>)) -> Self {
        Transaction::GenesisTransaction(GenesisTransaction {
            version: version.into(),
            hash: info.transaction_hash().into(),
            state_root_hash: info.state_root_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),

            data: bcs::to_bytes(&txn).unwrap_or_default().into(),
            events,
        })
    }
}

impl<T: TransactionInfoTrait> From<(u64, &BlockMetadata, &T)> for Transaction {
    fn from((version, txn, info): (u64, &BlockMetadata, &T)) -> Self {
        Transaction::BlockMetadataTransaction(BlockMetadataTransaction {
            version: version.into(),
            hash: info.transaction_hash().into(),
            state_root_hash: info.state_root_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),

            id: txn.id().into(),
            round: txn.round().into(),
            previous_block_votes: txn
                .previous_block_votes()
                .clone()
                .iter()
                .map(|a| (*a).into())
                .collect(),
            proposer: txn.proposer().into(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PendingTransaction {
    pub hash: HashValue,
    pub sender: Address,
    pub sequence_number: U64,
    pub max_gas_amount: U64,
    pub gas_unit_price: U64,
    pub gas_currency_code: String,
    pub expiration_timestamp_secs: U64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UserTransaction {
    pub version: U64,
    pub hash: HashValue,
    pub state_root_hash: HashValue,
    pub event_root_hash: HashValue,
    pub gas_used: U64,
    pub success: bool,

    // user txn specific fields
    pub sender: Address,
    pub sequence_number: U64,
    pub max_gas_amount: U64,
    pub gas_unit_price: U64,
    pub gas_currency_code: String,
    pub expiration_timestamp_secs: U64,
    pub events: Vec<Event>,
    pub payload: TransactionPayload,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GenesisTransaction {
    pub version: U64,
    pub hash: HashValue,
    pub state_root_hash: HashValue,
    pub event_root_hash: HashValue,
    pub gas_used: U64,
    pub success: bool,

    // genesis txn specific fields
    pub data: HexEncodedBytes,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BlockMetadataTransaction {
    pub version: U64,
    pub hash: HashValue,
    pub state_root_hash: HashValue,
    pub event_root_hash: HashValue,
    pub gas_used: U64,
    pub success: bool,

    // block metadata txn specific fields
    pub id: HashValue,
    pub round: U64,
    pub previous_block_votes: Vec<Address>,
    pub proposer: Address,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Event {
    pub key: EventKey,
    pub sequence_number: U64,
    pub transaction_version: U64,
    #[serde(rename = "type")]
    pub typ: MoveType,
    pub data: MoveValue,
}

impl From<(u64, &ContractEvent, AnnotatedMoveValue)> for Event {
    fn from((txn_version, event, data): (u64, &ContractEvent, AnnotatedMoveValue)) -> Self {
        match event {
            ContractEvent::V0(v0) => Self {
                key: (*v0.key()).into(),
                sequence_number: v0.sequence_number().into(),
                transaction_version: txn_version.into(),
                typ: v0.type_tag().clone().into(),
                data: data.into(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransactionPayload {
    ScriptFunctionPayload {
        module: MoveModuleId,
        function: Identifier,
        type_arguments: Vec<MoveType>,
        arguments: Vec<MoveValue>,
    },
    ScriptPayload(ScriptPayload),
    ModulePayload,
    WriteSetPayload,
}

impl From<&Script> for TransactionPayload {
    fn from(script: &Script) -> Self {
        TransactionPayload::ScriptPayload(ScriptPayload {
            code: script.code().to_vec().into(),
            type_arguments: script
                .ty_args()
                .iter()
                .map(|arg| arg.clone().into())
                .collect(),
            arguments: script.args().iter().map(|arg| arg.clone().into()).collect(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScriptPayload {
    pub code: HexEncodedBytes,
    pub type_arguments: Vec<MoveType>,
    pub arguments: Vec<MoveValue>,
}
