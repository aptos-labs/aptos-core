// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Address, EventKey, HashValue, HexEncodedBytes, MoveModule, MoveModuleId, MoveResource,
    MoveResourceType, MoveType, MoveValue, U64,
};

use diem_crypto::hash::CryptoHash;
use diem_types::{
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    transaction::{Script, SignedTransaction, TransactionInfoTrait},
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

impl From<(SignedTransaction, TransactionPayload)> for Transaction {
    fn from((txn, payload): (SignedTransaction, TransactionPayload)) -> Self {
        Transaction::PendingTransaction(PendingTransaction {
            sender: txn.sender().into(),
            sequence_number: txn.sequence_number().into(),
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            gas_currency_code: txn.gas_currency_code().to_owned(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs().into(),
            hash: Transaction::hash(txn),
            payload,
        })
    }
}

impl<T: TransactionInfoTrait> From<(u64, &SignedTransaction, &T, TransactionPayload, Vec<Event>)>
    for Transaction
{
    fn from(
        (version, txn, info, payload, events): (
            u64,
            &SignedTransaction,
            &T,
            TransactionPayload,
            Vec<Event>,
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

impl<T: TransactionInfoTrait> From<(u64, &T, WriteSetPayload, Vec<Event>)> for Transaction {
    fn from((version, info, payload, events): (u64, &T, WriteSetPayload, Vec<Event>)) -> Self {
        Transaction::GenesisTransaction(GenesisTransaction {
            version: version.into(),
            hash: info.transaction_hash().into(),
            state_root_hash: info.state_root_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),

            payload: GenesisPayload::WriteSetPayload(payload),
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
    pub payload: TransactionPayload,
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
    pub payload: TransactionPayload,
    pub events: Vec<Event>,
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
    pub payload: GenesisPayload,
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
    #[serde(rename = "type")]
    pub typ: MoveType,
    pub data: MoveValue,
}

impl From<(&ContractEvent, AnnotatedMoveValue)> for Event {
    fn from((event, data): (&ContractEvent, AnnotatedMoveValue)) -> Self {
        match event {
            ContractEvent::V0(v0) => Self {
                key: (*v0.key()).into(),
                sequence_number: v0.sequence_number().into(),
                typ: v0.type_tag().clone().into(),
                data: data.into(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenesisPayload {
    WriteSetPayload(WriteSetPayload),
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
    ModulePayload {
        code: HexEncodedBytes,
    },
    WriteSetPayload(WriteSetPayload),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScriptPayload {
    pub code: HexEncodedBytes,
    pub type_arguments: Vec<MoveType>,
    pub arguments: Vec<MoveValue>,
}

impl From<&Script> for ScriptPayload {
    fn from(script: &Script) -> Self {
        Self {
            code: script.code().to_vec().into(),
            type_arguments: script
                .ty_args()
                .iter()
                .map(|arg| arg.clone().into())
                .collect(),
            arguments: script.args().iter().map(|arg| arg.clone().into()).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WriteSetPayload {
    ScriptWriteSet {
        execute_as: Address,
        script: ScriptPayload,
    },
    DirectWriteSet {
        changes: Vec<WriteSetChange>,
        events: Vec<Event>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
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
        data: MoveModule,
    },
    WriteResource {
        address: Address,
        data: MoveResource,
    },
}
