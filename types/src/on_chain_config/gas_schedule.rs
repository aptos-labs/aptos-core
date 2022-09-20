// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GasSchedule {
    pub entries: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GasScheduleV2 {
    pub feature_version: u64,
    pub entries: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct StorageGasSchedule {
    pub per_item_read: u64,
    pub per_item_create: u64,
    pub per_item_write: u64,
    pub per_byte_read: u64,
    pub per_byte_create: u64,
    pub per_byte_write: u64,
}

impl GasSchedule {
    pub fn to_btree_map(self) -> BTreeMap<String, u64> {
        // TODO: what if the gas schedule contains duplicated entries?
        self.entries.into_iter().collect()
    }
}

impl GasScheduleV2 {
    pub fn to_btree_map(self) -> BTreeMap<String, u64> {
        // TODO: what if the gas schedule contains duplicated entries?
        self.entries.into_iter().collect()
    }
}

impl OnChainConfig for GasSchedule {
    const MODULE_IDENTIFIER: &'static str = "gas_schedule";
    const TYPE_IDENTIFIER: &'static str = "GasSchedule";
}

impl OnChainConfig for GasScheduleV2 {
    const MODULE_IDENTIFIER: &'static str = "gas_schedule";
    const TYPE_IDENTIFIER: &'static str = "GasScheduleV2";
}

impl OnChainConfig for StorageGasSchedule {
    const MODULE_IDENTIFIER: &'static str = "storage_gas";
    const TYPE_IDENTIFIER: &'static str = "StorageGas";
}
