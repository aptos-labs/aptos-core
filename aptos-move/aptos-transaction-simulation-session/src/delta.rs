// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::HumanReadable;
use anyhow::Result;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

// TODO: Store state values in human readable format

/// Saves the state delta to a file.
pub fn save_delta(delta_path: &Path, delta: &HashMap<StateKey, Option<StateValue>>) -> Result<()> {
    // Use BTreeMap to ensure deterministic ordering
    let mut delta_str = BTreeMap::new();

    for (k, v) in delta {
        let key_str = HumanReadable(k).to_string();
        let val_str_opt = match v {
            Some(v) => Some(hex::encode(&bcs::to_bytes(&v)?)),
            None => None,
        };
        delta_str.insert(key_str, val_str_opt);
    }

    let json = serde_json::to_string_pretty(&delta_str)?;
    std::fs::write(delta_path, json)?;

    Ok(())
}

/// Loads the state delta from a file.
pub fn load_delta(delta_path: &Path) -> Result<HashMap<StateKey, Option<StateValue>>> {
    let json = std::fs::read_to_string(delta_path)?;
    let delta_str: HashMap<HumanReadable<StateKey>, Option<String>> = serde_json::from_str(&json)?;

    let mut delta = HashMap::new();

    for (k, v) in delta_str {
        let key = k.into_inner();
        let val = match v {
            Some(v) => Some(bcs::from_bytes(&hex::decode(v)?)?),
            None => None,
        };
        delta.insert(key, val);
    }

    Ok(delta)
}

#[test]
fn test_delta_roundtrip() -> Result<()> {
    use aptos_transaction_simulation::{
        DeltaStateStore, EmptyStateView, SimulationStateStore, GENESIS_CHANGE_SET_HEAD,
    };
    use tempfile::NamedTempFile;

    let state_store = DeltaStateStore::new_with_base(EmptyStateView);
    state_store.apply_write_set(GENESIS_CHANGE_SET_HEAD.write_set())?;

    let delta = state_store.delta();

    // Use a temporary file for the delta
    let temp_file = NamedTempFile::new()?;
    let delta_path = temp_file.path();

    save_delta(delta_path, &delta)?;
    let delta_loaded = load_delta(delta_path)?;
    assert_eq!(delta, delta_loaded);

    Ok(())
}
