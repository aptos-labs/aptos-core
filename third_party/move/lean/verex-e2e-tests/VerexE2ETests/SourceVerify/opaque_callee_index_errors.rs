// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A caller indexing a vector through a contract at a shape, beside an opaque
// callee whose own proof uses the same contract.

pub struct Entry {
    pub key: u64,
    pub value: u64,
}

pub struct Map {
    pub entries: Vec<Entry>,
}

impl Map {
    fn find(&self, key: u64) -> Option<usize> {
        if self.entries.len() > 0 && self.entries[0].key == key {
            Some(0)
        } else {
            None
        }
    }

    pub fn get_or_zero(&self, key: u64) -> u64 {
        match self.find(key) {
            Some(i) => self.entries[i].value,
            None => 0,
        }
    }

    // False: `find` promises nothing about `i + 1`.
    pub fn next_or_zero(&self, key: u64) -> u64 {
        match self.find(key) {
            Some(i) => self.entries[i + 1].value,
            None => 0,
        }
    }

    // False: an entry's value need not be its key.
    pub fn value_is_key(&self, key: u64) -> u64 {
        match self.find(key) {
            Some(i) => self.entries[i].value,
            None => key,
        }
    }
}
