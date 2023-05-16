// Copyright © Aptos Foundation

use aptos_crypto::HashValue;
use std::collections::HashMap;
use crate::dag::dag_storage::ItemId;
use crate::dag::types::MissingDagNodeStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeStatusMap {
    pub(crate) id: ItemId,
    pub(crate) inner: HashMap<HashValue, MissingDagNodeStatus>,
}

impl MissingNodeStatusMap {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, k: &HashValue) -> Option<&MissingDagNodeStatus> {
        self.inner.get(k)
    }

    pub(crate) fn entry(&mut self, k: HashValue) -> std::collections::hash_map::Entry<'_, HashValue, MissingDagNodeStatus> {
        self.inner.entry(k)
    }

    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<'_, HashValue, MissingDagNodeStatus> {
        self.inner.iter()
    }

    pub(crate) fn insert(&mut self, k: HashValue, v: MissingDagNodeStatus) -> Option<MissingDagNodeStatus> {
        self.inner.insert(k, v)
    }
}

////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeStatusMapEntry_Key {
    pub(crate) map_id: ItemId,
    pub(crate) key: Option<HashValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeStatusMapEntry {
    pub(crate) map_id: ItemId,
    pub(crate) key: Option<HashValue>,
    pub(crate) value: Option<MissingDagNodeStatus>,
}

impl MissingNodeStatusMapEntry {
    pub(crate) fn key(&self) -> MissingNodeStatusMapEntry_Key {
        MissingNodeStatusMapEntry_Key {
            map_id: self.map_id,
            key: self.key,
        }
    }
}
