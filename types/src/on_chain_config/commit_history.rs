// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{on_chain_config::OnChainConfig, state_store::table::TableHandle};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct TableWithLength {
    handle: TableHandle,
    length: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CommitHistory {
    max_capacity: u32,
    current_idx: u32,
    table: TableWithLength,
}

impl OnChainConfig for CommitHistory {
    const MODULE_IDENTIFIER: &'static str = "block";
    const TYPE_IDENTIFIER: &'static str = "CommitHistory";
}
