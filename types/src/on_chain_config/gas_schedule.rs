// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};
use std::collections::{btree_map, BTreeMap};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GasSchedule {
    pub entries: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GasScheduleV2 {
    pub feature_version: u64,
    pub entries: Vec<(String, u64)>,
}

#[derive(Debug)]
pub enum DiffItem<T> {
    Add { new_val: T },
    Delete { old_val: T },
    Modify { old_val: T, new_val: T },
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

impl StorageGasSchedule {
    pub fn zeros() -> Self {
        Self {
            per_item_read: 0,
            per_item_create: 0,
            per_item_write: 0,
            per_byte_read: 0,
            per_byte_create: 0,
            per_byte_write: 0,
        }
    }
}

impl GasSchedule {
    pub fn into_btree_map(self) -> BTreeMap<String, u64> {
        // TODO: what if the gas schedule contains duplicated entries?
        self.entries.into_iter().collect()
    }
}

impl GasScheduleV2 {
    pub fn into_btree_map(self) -> BTreeMap<String, u64> {
        // TODO: what if the gas schedule contains duplicated entries?
        self.entries.into_iter().collect()
    }

    pub fn to_btree_map_borrowed(&self) -> BTreeMap<&str, u64> {
        self.entries.iter().map(|(k, v)| (k.as_str(), *v)).collect()
    }

    pub fn diff<'a>(old: &'a Self, new: &'a Self) -> BTreeMap<&'a str, DiffItem<u64>> {
        let mut old = old.to_btree_map_borrowed();
        let new = new.to_btree_map_borrowed();

        let mut diff = BTreeMap::new();
        for (param_name, new_val) in new {
            match old.entry(param_name) {
                btree_map::Entry::Occupied(entry) => {
                    let (param_name, old_val) = entry.remove_entry();

                    if old_val != new_val {
                        diff.insert(param_name, DiffItem::Modify { old_val, new_val });
                    }
                },
                btree_map::Entry::Vacant(entry) => {
                    let param_name = entry.into_key();
                    diff.insert(param_name, DiffItem::Add { new_val });
                },
            }
        }
        diff.extend(
            old.into_iter()
                .map(|(param_name, old_val)| (param_name, DiffItem::Delete { old_val })),
        );

        diff
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
