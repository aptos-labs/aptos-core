// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{contract_event::ContractEvent, event::EventKey, on_chain_config::new_epoch_event_key};
use anyhow::Result;
use move_core_types::{
    ident_str, identifier::IdentStr, language_storage::TypeTag, move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Struct that represents a NewEpochEvent.
#[derive(Debug, Serialize, Deserialize)]
pub struct NewEpochEvent {
    epoch: u64,
}

impl NewEpochEvent {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self { epoch: 0 }
    }

    pub fn as_contract_event(&self, seq_num: u64) -> ContractEvent {
        ContractEvent::new_v1(
            new_epoch_event_key(),
            seq_num,
            TypeTag::from_str("0x1::reconfiguration::NewEpochEvent").unwrap(),
            bcs::to_bytes(self).unwrap(),
        )
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn event_key() -> EventKey {
        crate::on_chain_config::new_epoch_event_key()
    }
}

impl MoveStructType for NewEpochEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("reconfiguration");
    const STRUCT_NAME: &'static IdentStr = ident_str!("NewEpochEvent");
}

pub static NEW_EPOCH_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(NewEpochEvent::struct_tag())));
