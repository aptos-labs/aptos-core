// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};
use std::collections::{btree_map, BTreeMap};

const KEY_MIN_PRICE_PER_GAS: &str = "txn.min_price_per_gas_unit";

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

    pub fn from_json_string(json_str: String) -> Self {
        serde_json::from_str(&json_str).unwrap()
    }

    pub fn scale_min_gas_price_by(&mut self, factor: f64) {
        for (name, value) in &mut self.entries {
            if name == KEY_MIN_PRICE_PER_GAS {
                // Convert to f64, multiply, then convert back to u64 with saturation
                let new_value = (*value as f64 * factor).round() as u64;
                *value = new_value;
            }
        }
    }


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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_min_gas_price_by() {
        let mut gas_schedule = GasScheduleV2 {
            feature_version: 1,
            entries: vec![
                ("other.param".to_string(), 100),
                (KEY_MIN_PRICE_PER_GAS.to_string(), 50),
                ("another.param".to_string(), 200),
            ],
        };

        gas_schedule.scale_min_gas_price_by(0.5);

        // Check that only the min gas price was scaled
        assert_eq!(gas_schedule.entries[0], ("other.param".to_string(), 100));
        assert_eq!(gas_schedule.entries[1], (KEY_MIN_PRICE_PER_GAS.to_string(), 25));
        assert_eq!(gas_schedule.entries[2], ("another.param".to_string(), 200));

        gas_schedule.scale_min_gas_price_by(2.0);
        assert_eq!(gas_schedule.entries[1], (KEY_MIN_PRICE_PER_GAS.to_string(), 50));

        gas_schedule.scale_min_gas_price_by(10.0);
        assert_eq!(gas_schedule.entries[1], (KEY_MIN_PRICE_PER_GAS.to_string(), 500));

        gas_schedule.scale_min_gas_price_by(0.1);
        assert_eq!(gas_schedule.entries[1], (KEY_MIN_PRICE_PER_GAS.to_string(), 500));
    }

    #[test]
    fn test_scale_min_gas_price_by_rounding() {
        let mut gas_schedule = GasScheduleV2 {
            feature_version: 1,
            entries: vec![
                (KEY_MIN_PRICE_PER_GAS.to_string(), 100),
            ],
        };

        // Test rounding behavior (100 * 1.234 = 123.4, should round to 123)
        gas_schedule.scale_min_gas_price_by(1.234);
        assert_eq!(gas_schedule.entries[0].1, 123);
    }

    #[test]
    fn test_scale_min_gas_price_by_overflow_protection() {
        let mut gas_schedule = GasScheduleV2 {
            feature_version: 1,
            entries: vec![
                (KEY_MIN_PRICE_PER_GAS.to_string(), u64::MAX),
            ],
        };

        // Should not panic with large factor
        gas_schedule.scale_min_gas_price_by(2.0);
        // The result will be clamped to u64::MAX due to overflow in f64 to u64 conversion
        println!("{:?}", gas_schedule);
        assert!(gas_schedule.entries[0].1 > 0);
    }
}
