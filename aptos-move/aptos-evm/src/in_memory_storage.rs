// Copyright © Aptos Foundation

use anyhow::Error;
use aptos_table_natives::{TableHandle, TableResolver};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct InMemoryTableResolver {
    tables: BTreeMap<TableHandle, BTreeMap<Vec<u8>, Vec<u8>>>,
}

impl InMemoryTableResolver {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
        }
    }

    pub fn add_table(&mut self, handle: TableHandle) {
        self.tables.insert(handle, BTreeMap::new());
    }

    pub fn add_table_entry(&mut self, handle: &TableHandle, key: Vec<u8>, value: Vec<u8>) {
        self.tables.get_mut(handle).unwrap().insert(key, value);
    }
}

impl TableResolver for InMemoryTableResolver {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> std::result::Result<Option<Vec<u8>>, Error> {
        Ok(self.tables.get(handle).and_then(|t| t.get(key).cloned()))
    }
}
