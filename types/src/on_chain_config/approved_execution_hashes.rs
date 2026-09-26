// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

    /// Whether governance has approved a script with this hash.
    pub fn contains_script_hash(&self, script_hash: &[u8]) -> bool {
        self.entries
            .iter()
            .any(|(_, hash)| hash.as_slice() == script_hash)
    }
}

impl OnChainConfig for ApprovedExecutionHashes {
    const MODULE_IDENTIFIER: &'static str = "aptos_governance";
    const TYPE_IDENTIFIER: &'static str = "ApprovedExecutionHashes";
}
