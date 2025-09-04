// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{contract_event::ContractEvent, event::EventHandle};
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use serde::Serialize;

pub trait MoveEventV1Type: MoveStructType + Serialize {
    fn create_event_v1(&self, handle: &mut EventHandle) -> ContractEvent {
        let sequence_number = handle.count();
        *handle.count_mut() = sequence_number + 1;
        ContractEvent::new_v1(
            *handle.key(),
            sequence_number,
            TypeTag::Struct(Box::new(Self::struct_tag())),
            bcs::to_bytes(self).unwrap(),
        )
        .unwrap()
    }
}
