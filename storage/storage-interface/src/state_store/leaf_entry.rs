// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    state_store::{state_key::StateKey, state_slot::StateSlot, state_value::StateValue},
    transaction::Version,
};

/// Read-shape for any leaf-style slot. `state_key()` is `Option`
/// because main state's `StateSlot` can be loaded from the hot KV DB
/// with only the key hash.
pub trait LeafEntry: Clone {
    type Value;

    fn state_key(&self) -> Option<&StateKey>;
    fn value(&self) -> Option<&Self::Value>;
    fn value_hash(&self) -> Option<HashValue>;

    /// Filter for `merklize_snapshot`: when `false` the slot is skipped.
    /// Main state filters by `value_version >= min_version`; others
    /// rely on the caller having already filtered the delta.
    fn passes_jmt_filter(&self, _min_version: Version) -> bool {
        true
    }
}

impl LeafEntry for StateSlot {
    type Value = StateValue;

    fn state_key(&self) -> Option<&StateKey> {
        StateSlot::state_key(self)
    }

    fn value(&self) -> Option<&StateValue> {
        self.as_state_value_opt()
    }

    fn value_hash(&self) -> Option<HashValue> {
        self.as_state_value_opt().map(CryptoHash::hash)
    }

    fn passes_jmt_filter(&self, min_version: Version) -> bool {
        StateSlot::passes_jmt_filter(self, min_version)
    }
}

/// Inner `None` means a JMT delete (vacant slot). An occupied slot
/// without a `state_key` is an invariant violation and panics.
pub fn leaf_entry_to_jmt_update<S: LeafEntry>(
    key_hash: HashValue,
    slot: &S,
) -> (HashValue, Option<(HashValue, StateKey)>) {
    let leaf = slot.value_hash().map(|h| {
        let k = slot
            .state_key()
            .expect("occupied leaf slot must carry a state_key");
        (h, k.clone())
    });
    (key_hash, leaf)
}
