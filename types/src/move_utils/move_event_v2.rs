// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::contract_event::ContractEvent;
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use serde::Serialize;

pub trait MoveEventV2Type: MoveStructType + Serialize {
    fn create_event_v2(&self) -> anyhow::Result<ContractEvent> {
        ContractEvent::new_v2(
            TypeTag::Struct(Box::new(Self::struct_tag())),
            bcs::to_bytes(self)?,
        )
    }
}
