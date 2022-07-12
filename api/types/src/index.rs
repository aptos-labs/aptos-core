// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::LedgerInfo;
use aptos_config::config::RoleType;
use serde::{Deserialize, Serialize};
use utoipa::Component;

/// The struct holding all data returned to the client by the
/// index endpoint (i.e., GET "/"). The data is flattened into
/// a single JSON map to offer easier parsing for clients.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Component)]
pub struct IndexResponse {
    #[serde(flatten)]
    pub ledger_info: LedgerInfo,
    pub node_role: RoleType,
}

impl IndexResponse {
    pub fn new(ledger_info: LedgerInfo, node_role: RoleType) -> IndexResponse {
        Self {
            ledger_info,
            node_role,
        }
    }
}
