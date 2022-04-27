// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, HexEncodedBytes, U64};

use aptos_types::{
    account_config::{AccountResource, TransferEvents as TransferEventsStruct},
    event::EventKey,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountData {
    pub sequence_number: U64,
    pub authentication_key: HexEncodedBytes,
    pub self_address: Address,
    pub balance: TestCoin,
    pub transfer_events: TransferEvents,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestCoin {
    pub value: U64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferEvents {
    pub sent_events: EventHandle,
    pub received_events: EventHandle,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventHandle {
    pub counter: U64,
    pub guid: EventHandleGUID,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventHandleGUID {
    len_bytes: u8,
    guid: GUID,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GUID {
    id: ID,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ID {
    creation_num: U64,
    addr: Address,
}

pub fn convert_event_key(event_key: &EventKey) -> EventHandleGUID {
    let len_bytes = 40u8;
    let creation_num = event_key.get_creation_number().into();
    let addr = event_key.get_creator_address().into();
    EventHandleGUID {
        len_bytes,
        guid: GUID {
            id: ID { creation_num, addr },
        },
    }
}

impl From<TransferEventsStruct> for TransferEvents {
    fn from(events: TransferEventsStruct) -> Self {
        let sent_events = events.sent_events();
        let received_events = events.received_events();
        Self {
            sent_events: EventHandle {
                counter: sent_events.count().into(),
                guid: convert_event_key(sent_events.key()),
            },
            received_events: EventHandle {
                counter: received_events.count().into(),
                guid: convert_event_key(received_events.key()),
            },
        }
    }
}

impl From<AccountResource> for AccountData {
    fn from(ar: AccountResource) -> Self {
        Self {
            sequence_number: ar.sequence_number().into(),
            authentication_key: ar.authentication_key().to_vec().into(),
            self_address: ar.address().into(),
            balance: TestCoin {
                value: ar.balance().into(),
            },
            transfer_events: ar.transfer_events().clone().into(),
        }
    }
}
