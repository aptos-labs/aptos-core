// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use serde::Serialize;

use crate::contract_event::ContractEvent;

pub trait MoveEventV2: MoveStructType + Serialize {
    fn create_event_v2(&self) -> ContractEvent {
        ContractEvent::new_v2(
            TypeTag::Struct(Box::new(Self::struct_tag())),
            bcs::to_bytes(self).unwrap()
        )
    }
}
