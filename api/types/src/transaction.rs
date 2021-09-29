// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, EventKey, HashValue, HexEncodedBytes, MoveType, MoveValue, U64};

use diem_types::{
    contract_event::ContractEvent,
    transaction::{Transaction as DiemTransaction, TransactionInfoTrait},
    vm_status::KeptVMStatus,
};
use resource_viewer::AnnotatedMoveValue;

use serde::Serialize;
use std::convert::From;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transaction {
    UserTransaction {
        version: U64,
        hash: HashValue,
        state_root_hash: HashValue,
        event_root_hash: HashValue,
        gas_used: U64,
        success: bool,

        // user txn specific fields
        sender: Address,
        sequence_number: U64,
        max_gas_amount: U64,
        gas_unit_price: U64,
        gas_currency_code: String,
        expiration_timestamp_secs: U64,
        events: Vec<Event>,
    },
    GenesisTransaction {
        version: U64,
        hash: HashValue,
        state_root_hash: HashValue,
        event_root_hash: HashValue,
        gas_used: U64,
        success: bool,

        // genesis txn specific fields
        data: HexEncodedBytes,
        events: Vec<Event>,
    },
    BlockMetadata {
        version: U64,
        hash: HashValue,
        state_root_hash: HashValue,
        event_root_hash: HashValue,
        gas_used: U64,
        success: bool,

        // block metadata txn specific fields
        id: HashValue,
        round: U64,
        previous_block_votes: Vec<Address>,
        proposer: Address,
    },
}

impl<T: TransactionInfoTrait> From<(u64, &DiemTransaction, &T, Vec<Event>)> for Transaction {
    fn from((version, submitted, info, events): (u64, &DiemTransaction, &T, Vec<Event>)) -> Self {
        match submitted {
            DiemTransaction::UserTransaction(txn) => Transaction::UserTransaction {
                version: version.into(),
                hash: info.transaction_hash().into(),
                state_root_hash: info.state_root_hash().into(),
                event_root_hash: info.event_root_hash().into(),
                gas_used: info.gas_used().into(),
                success: info.status() == &KeptVMStatus::Executed,

                sender: txn.sender().into(),
                sequence_number: txn.sequence_number().into(),
                max_gas_amount: txn.max_gas_amount().into(),
                gas_unit_price: txn.gas_unit_price().into(),
                gas_currency_code: txn.gas_currency_code().to_owned(),
                expiration_timestamp_secs: txn.expiration_timestamp_secs().into(),
                events,
            },
            DiemTransaction::GenesisTransaction(txn) => Transaction::GenesisTransaction {
                version: version.into(),
                hash: info.transaction_hash().into(),
                state_root_hash: info.state_root_hash().into(),
                event_root_hash: info.event_root_hash().into(),
                gas_used: info.gas_used().into(),
                success: info.status() == &KeptVMStatus::Executed,

                data: bcs::to_bytes(&txn).unwrap_or_default().into(),
                events,
            },
            DiemTransaction::BlockMetadata(txn) => Transaction::BlockMetadata {
                version: version.into(),
                hash: info.transaction_hash().into(),
                state_root_hash: info.state_root_hash().into(),
                event_root_hash: info.event_root_hash().into(),
                gas_used: info.gas_used().into(),
                success: info.status() == &KeptVMStatus::Executed,

                id: txn.id().into(),
                round: txn.round().into(),
                previous_block_votes: txn
                    .previous_block_votes()
                    .clone()
                    .iter()
                    .map(|a| (*a).into())
                    .collect(),
                proposer: txn.proposer().into(),
            },
        }
    }
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
