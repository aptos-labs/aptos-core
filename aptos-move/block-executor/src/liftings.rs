// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use dashmap::{mapref::one::Ref, DashMap};
use move_vm_types::values::Value;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

pub(crate) struct UnsyncLiftings {
    counter: u64,
    lifted_values: HashMap<u64, Value>,
}

impl UnsyncLiftings {
    pub(crate) fn new(counter: u64) -> Self {
        Self {
            counter,
            lifted_values: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, value: Value) -> u64 {
        let identifier = self.counter;
        self.lifted_values.insert(identifier, value);
        self.counter += 1;
        identifier
    }

    pub(crate) fn get(&self, identifier: u64) -> Option<&Value> {
        self.lifted_values.get(&identifier)
    }
}

pub(crate) struct SyncLiftings<'a> {
    counter: &'a AtomicU64,
    lifted_values: DashMap<u64, Value>,
}

impl<'a> SyncLiftings<'a> {
    pub(crate) fn new(shared_counter: &'a AtomicU64) -> Self {
        Self {
            counter: shared_counter,
            lifted_values: DashMap::new(),
        }
    }

    pub(crate) fn insert(&self, value: Value) -> u64 {
        let identifier = self.counter.fetch_add(1, Ordering::SeqCst);
        self.lifted_values.insert(identifier, value);
        identifier
    }

    pub(crate) fn get(&self, identifier: u64) -> Option<Ref<u64, Value>> {
        self.lifted_values.get(&identifier)
    }
}
