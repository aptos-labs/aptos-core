// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::{DepositEvent, NewBlockEvent, NewEpochEvent, WithdrawEvent},
    event::EventKey,
    transaction::Version,
};
use anyhow::{bail, Error, Result};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// This trait is used by block executor to abstractly represent an event.
/// Block executor uses `get_event_data` to get the event data.
/// Block executor then checks for the occurences of aggregators and aggregatorsnapshots
/// in the event data, processes them, and calls `update_event_data` to update the event data.
pub trait ReadWriteEvent {
    /// Returns the event data.
    fn get_event_data(&self) -> (EventKey, u64, &TypeTag, &[u8]);
    /// Updates the event data.
    fn update_event_data(&mut self, event_data: Vec<u8>);
}

/// Support versioning of the data structure.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ContractEvent {
    V0(ContractEventV0),
    V1(ContractEventV1),
}

impl ReadWriteEvent for ContractEvent {
    fn get_event_data(&self) -> (EventKey, u64, &TypeTag, &[u8]) {
        match self {
            ContractEvent::V0(event) => (
                *event.key(),
                event.sequence_number(),
                event.type_tag(),
                event.event_data(),
            ),
        }
    }

    fn update_event_data(&mut self, event_data: Vec<u8>) {
        match self {
            ContractEvent::V0(event) => event.event_data = event_data,
        }
    }
}

impl ContractEvent {
    pub fn new_v0(
        key: EventKey,
        sequence_number: u64,
        type_tag: TypeTag,
        event_data: Vec<u8>,
    ) -> Self {
        ContractEvent::V0(ContractEventV0::new(
            key,
            sequence_number,
            type_tag,
            event_data,
        ))
    }

    pub fn new_v1(type_tag: TypeTag, event_data: Vec<u8>) -> Self {
        ContractEvent::V1(ContractEventV1::new(type_tag, event_data))
    }

    pub fn event_data(&self) -> &[u8] {
        match self {
            ContractEvent::V0(event) => event.event_data(),
            ContractEvent::V1(event) => event.event_data(),
        }
    }

    pub fn type_tag(&self) -> &TypeTag {
        match self {
            ContractEvent::V0(event) => &event.type_tag,
            ContractEvent::V1(event) => &event.type_tag,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            ContractEvent::V0(event) => event.size(),
            ContractEvent::V1(event) => event.size(),
        }
    }

    pub fn is_v0(&self) -> bool {
        matches!(self, ContractEvent::V0(_))
    }

    pub fn is_v1(&self) -> bool {
        matches!(self, ContractEvent::V1(_))
    }

    pub fn v0(&self) -> Result<&ContractEventV0> {
        Ok(match self {
            ContractEvent::V0(event) => event,
            ContractEvent::V1(_event) => bail!("This is a module event"),
        })
    }

    pub fn v1(&self) -> Result<&ContractEventV1> {
        Ok(match self {
            ContractEvent::V0(_event) => bail!("This is a instance event"),
            ContractEvent::V1(event) => event,
        })
    }
}

/// Entry produced via a call to the `emit_event` builtin.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, CryptoHasher)]
pub struct ContractEventV0 {
    /// The unique key that the event was emitted to
    key: EventKey,
    /// The number of messages that have been emitted to the path previously
    sequence_number: u64,
    /// The type of the data
    type_tag: TypeTag,
    /// The data payload of the event
    #[serde(with = "serde_bytes")]
    event_data: Vec<u8>,
}

/// Entry produced via a call to the `emit` builtin.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, CryptoHasher)]
pub struct ContractEventV1 {
    /// The type of the data
    type_tag: TypeTag,
    /// The data payload of the event
    #[serde(with = "serde_bytes")]
    event_data: Vec<u8>,
}

impl ContractEventV0 {
    pub fn new(
        key: EventKey,
        sequence_number: u64,
        type_tag: TypeTag,
        event_data: Vec<u8>,
    ) -> Self {
        Self {
            key,
            sequence_number,
            type_tag,
            event_data,
        }
    }

    pub fn key(&self) -> &EventKey {
        &self.key
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn event_data(&self) -> &[u8] {
        &self.event_data
    }

    pub fn type_tag(&self) -> &TypeTag {
        &self.type_tag
    }

    pub fn size(&self) -> usize {
        self.key.size() + 8 /* u64 */ + bcs::to_bytes(&self.type_tag).unwrap().len() + self.event_data.len()
    }
}

impl std::fmt::Debug for ContractEventV0 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ContractEvent {{ key: {:?}, index: {:?}, type: {:?}, event_data: {:?} }}",
            self.key,
            self.sequence_number,
            self.type_tag,
            hex::encode(&self.event_data)
        )
    }
}

impl ContractEventV1 {
    pub fn new(type_tag: TypeTag, event_data: Vec<u8>) -> Self {
        Self {
            type_tag,
            event_data,
        }
    }

    pub fn size(&self) -> usize {
        bcs::to_bytes(&self.type_tag).unwrap().len() + self.event_data.len()
    }

    pub fn type_tag(&self) -> &TypeTag {
        &self.type_tag
    }

    pub fn event_data(&self) -> &[u8] {
        &self.event_data
    }
}

impl std::fmt::Debug for ContractEventV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ModuleEvent {{ type: {:?}, event_data: {:?} }}",
            self.type_tag,
            hex::encode(&self.event_data)
        )
    }
}

impl TryFrom<&ContractEvent> for NewBlockEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V0(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    anyhow::bail!("Expected NewBlockEvent")
                }
                Self::try_from_bytes(&event.event_data)
            },
            ContractEvent::V1(_) => anyhow::bail!("This is a module event"),
        }
    }
}

impl TryFrom<&ContractEvent> for NewEpochEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V0(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    anyhow::bail!("Expected NewEpochEvent")
                }
                Self::try_from_bytes(&event.event_data)
            },
            ContractEvent::V1(_) => anyhow::bail!("This is a module event"),
        }
    }
}

impl TryFrom<&ContractEvent> for WithdrawEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V0(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    anyhow::bail!("Expected Sent Payment")
                }
                Self::try_from_bytes(&event.event_data)
            },
            ContractEvent::V1(_) => anyhow::bail!("This is a module event"),
        }
    }
}

impl TryFrom<&ContractEvent> for DepositEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V0(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    anyhow::bail!("Expected Received Payment")
                }
                Self::try_from_bytes(&event.event_data)
            },
            ContractEvent::V1(_) => anyhow::bail!("This is a module event"),
        }
    }
}

impl std::fmt::Debug for ContractEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractEvent::V0(event) => event.fmt(f),
            ContractEvent::V1(event) => event.fmt(f),
        }
    }
}

impl std::fmt::Display for ContractEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let ContractEvent::V0(v0) = self {
            if let Ok(payload) = WithdrawEvent::try_from(self) {
                return write!(
                    f,
                    "ContractEvent {{ key: {}, index: {:?}, type: {:?}, event_data: {:?} }}",
                    v0.key, v0.sequence_number, v0.type_tag, payload,
                );
            } else if let Ok(payload) = DepositEvent::try_from(self) {
                return write!(
                    f,
                    "ContractEvent {{ key: {}, index: {:?}, type: {:?}, event_data: {:?} }}",
                    v0.key, v0.sequence_number, v0.type_tag, payload,
                );
            }
        }
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct EventWithVersion {
    pub transaction_version: u64,
    // Should be `Version`
    pub event: ContractEvent,
}

impl std::fmt::Display for EventWithVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventWithVersion {{ \n\ttransaction_version: {}, \n\tevent: {} \n}}",
            self.transaction_version, self.event
        )
    }
}

impl EventWithVersion {
    /// Constructor.
    pub fn new(transaction_version: Version, event: ContractEvent) -> Self {
        Self {
            transaction_version,
            event,
        }
    }
}
