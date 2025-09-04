// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{on_chain_config::OnChainConfig, state_store::table::TableHandle};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct TableWithLength {
    handle: TableHandle,
    length: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CommitHistoryResource {
    max_capacity: u32,
    next_idx: u32,
    table: TableWithLength,
}

impl CommitHistoryResource {
    pub fn max_capacity(&self) -> u32 {
        self.max_capacity
    }

    pub fn next_idx(&self) -> u32 {
        self.next_idx
    }

    pub fn table_handle(&self) -> &TableHandle {
        &self.table.handle
    }

    pub fn length(&self) -> u64 {
        self.table.length
    }
}

impl OnChainConfig for CommitHistoryResource {
    const MODULE_IDENTIFIER: &'static str = "block";
    const TYPE_IDENTIFIER: &'static str = "CommitHistory";
}
