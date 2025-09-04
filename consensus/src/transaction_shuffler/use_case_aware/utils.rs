// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

pub(crate) trait StrictMap<K, V> {
    fn strict_insert(&mut self, key: K, value: V);

    fn strict_remove(&mut self, key: &K);

    fn expect_mut(&mut self, key: &K) -> &mut V;
}

impl<K: Eq + Hash, V> StrictMap<K, V> for HashMap<K, V> {
    fn strict_insert(&mut self, key: K, value: V) {
        assert!(self.insert(key, value).is_none())
    }

    fn strict_remove(&mut self, key: &K) {
        assert!(self.remove(key).is_some())
    }

    fn expect_mut(&mut self, key: &K) -> &mut V {
        self.get_mut(key).expect("Known to exist.")
    }
}

impl<K: Ord, V> StrictMap<K, V> for BTreeMap<K, V> {
    fn strict_insert(&mut self, key: K, value: V) {
        assert!(self.insert(key, value).is_none())
    }

    fn strict_remove(&mut self, key: &K) {
        assert!(self.remove(key).is_some())
    }

    fn expect_mut(&mut self, key: &K) -> &mut V {
        self.get_mut(key).expect("Known to exist.")
    }
}
