// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GasFeatureVersion {
    pub major: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GasSchedule {
    pub entries: Vec<(String, u64)>,
}

impl GasSchedule {
    pub fn to_btree_map(self) -> BTreeMap<String, u64> {
        // TODO: what if the gas schedule contains duplicated entries?
        self.entries.into_iter().collect()
    }
}

impl OnChainConfig for GasSchedule {
    const MODULE_IDENTIFIER: &'static str = "gas_schedule";
    const TYPE_IDENTIFIER: &'static str = "GasSchedule";
}

impl OnChainConfig for GasFeatureVersion {
    const MODULE_IDENTIFIER: &'static str = "gas_schedule";
    const TYPE_IDENTIFIER: &'static str = "GasFeatureVersion";
}
