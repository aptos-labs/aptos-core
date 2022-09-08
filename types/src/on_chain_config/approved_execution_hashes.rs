// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ApprovedExecutionHashes {
    pub entries: Vec<(u64, Vec<u8>)>,
}

impl ApprovedExecutionHashes {
    pub fn to_btree_map(self) -> BTreeMap<u64, Vec<u8>> {
        self.entries.into_iter().collect()
    }
}

impl OnChainConfig for ApprovedExecutionHashes {
    const MODULE_IDENTIFIER: &'static str = "aptos_governance";
    const TYPE_IDENTIFIER: &'static str = "ApprovedExecutionHashes";
}
