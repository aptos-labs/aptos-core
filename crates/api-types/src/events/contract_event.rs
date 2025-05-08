// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::events::new_epoch::NewEpochEvent;
use anyhow::{bail, Error, Result};
// use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{convert::TryFrom, ops::Deref, str::FromStr};


/// Support versioning of the data structure.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContractEvent {
    V2(ContractEventV2),
}


impl ContractEvent {
    // pub fn new_v2(type_tag: TypeTag, event_data: Vec<u8>) -> Self {
    //     ContractEvent::V2(ContractEventV2::new(type_tag, event_data))
    // }

    // pub fn new_v2_with_type_tag_str(type_tag_str: &str, event_data: Vec<u8>) -> Self {
    //     ContractEvent::V2(ContractEventV2::new(
    //         TypeTag::from_str(type_tag_str).unwrap(),
    //         event_data,
    //     ))
    // }

    pub fn event_data(&self) -> &[u8] {
        match self {
            ContractEvent::V2(event) => event.event_data(),
        }
    }

    // pub fn type_tag(&self) -> &TypeTag {
    //     match self {
    //         ContractEvent::V2(event) => &event.type_tag,
    //     }
    // }

    pub fn size(&self) -> usize {
        match self {
            ContractEvent::V2(event) => event.size(),
        }
    }

    pub fn is_v2(&self) -> bool {
        matches!(self, ContractEvent::V2(_))
    }

    pub fn v2(&self) -> Result<&ContractEventV2> {
        Ok(match self {
            ContractEvent::V2(event) => event,
        })
    }

    pub fn try_v2(&self) -> Option<&ContractEventV2> {
        match self {
            ContractEvent::V2(event) => Some(event),
        }
    }

    // pub fn try_v2_typed<T: DeserializeOwned>(&self, event_type: &TypeTag) -> Result<Option<T>> {
    //     if let Some(v2) = self.try_v2() {
    //         if &v2.type_tag == event_type {
    //             return Ok(Some(bcs::from_bytes(&v2.event_data)?));
    //         }
    //     }

    //     Ok(None)
    // }

    // pub fn is_new_epoch_event(&self) -> bool {
    //     self.type_tag() == NEW_EPOCH_EVENT_MOVE_TYPE_TAG.deref()
    //         || self.type_tag() == NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG.deref()
    // }

}

/// Entry produced via a call to the `emit` builtin.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContractEventV2 {
    /// The type of the data
    // type_tag: TypeTag,
    /// The data payload of the event
    #[serde(with = "serde_bytes")]
    event_data: Vec<u8>,
}

impl ContractEventV2 {
    pub fn new(event_data: Vec<u8>) -> Self {
        Self {
            event_data,
        }
    }

    pub fn size(&self) -> usize {
        self.event_data.len()
    }

    // pub fn type_tag(&self) -> &TypeTag {
    //     &self.type_tag
    // }

    pub fn event_data(&self) -> &[u8] {
        &self.event_data
    }
}

impl std::fmt::Debug for ContractEventV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ModuleEvent {{ event_data: {:?} }}",
            hex::encode(&self.event_data)
        )
    }
}

// impl TryFrom<&ContractEvent> for NewBlockEvent {
//     type Error = Error;

//     fn try_from(event: &ContractEvent) -> Result<Self> {
//         match event {
//             ContractEvent::V1(event) => {
//                 if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
//                     bail!("Expected NewBlockEvent")
//                 }
//                 Self::try_from_bytes(&event.event_data)
//             },
//             ContractEvent::V2(_) => bail!("This is a module event"),
//         }
//     }
// }

impl TryFrom<&ContractEvent> for NewEpochEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        Self::try_from_bytes(event.event_data())
        // if event.is_new_epoch_event() {
        //     Self::try_from_bytes(event.event_data())
        // } else {
        //     bail!("Expected NewEpochEvent")
        // }
    }
}



// impl TryFrom<&ContractEvent> for ObservedJWKsUpdated {
//     type Error = Error;

//     fn try_from(event: &ContractEvent) -> Result<Self> {
//         match event {
//             ContractEvent::V1(_) => {
//                 bail!("conversion to `ObservedJWKsUpdated` failed with wrong event version")
//             },
//             ContractEvent::V2(v2) => {
//                 if v2.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
//                     bail!("conversion to `ObservedJWKsUpdated` failed with wrong type tag");
//                 }
//                 bcs::from_bytes(&v2.event_data).map_err(Into::into)
//             },
//         }
//     }
// }

impl std::fmt::Debug for ContractEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractEvent::V2(event) => event.fmt(f),
        }
    }
}

// impl std::fmt::Display for ContractEvent {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         if let Ok(payload) = WithdrawEvent::try_from(self) {
//             let v1 = self.v1().unwrap();
//             write!(
//                 f,
//                 "ContractEvent {{ key: {}, index: {:?}, type: {:?}, event_data: {:?} }}",
//                 v1.key, v1.sequence_number, v1.type_tag, payload,
//             )
//         } else if let Ok(payload) = DepositEvent::try_from(self) {
//             let v1 = self.v1().unwrap();
//             write!(
//                 f,
//                 "ContractEvent {{ key: {}, index: {:?}, type: {:?}, event_data: {:?} }}",
//                 v1.key, v1.sequence_number, v1.type_tag, payload,
//             )
//         } else {
//             write!(f, "{:?}", self)
//         }
//     }
// }

// #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
// #[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
// pub struct EventWithVersion {
//     pub transaction_version: Version,
//     pub event: ContractEvent,
// }

// impl std::fmt::Display for EventWithVersion {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "EventWithVersion {{ \n\ttransaction_version: {}, \n\tevent: {} \n}}",
//             self.transaction_version, self.event
//         )
//     }
// }

// impl EventWithVersion {
//     /// Constructor.
//     pub fn new(transaction_version: Version, event: ContractEvent) -> Self {
//         Self {
//             transaction_version,
//             event,
//         }
//     }
// }
