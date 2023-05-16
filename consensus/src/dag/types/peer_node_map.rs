// Copyright © Aptos Foundation

use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress as PeerId;
use std::collections::HashMap;
use aptos_consensus_types::node::CertifiedNode;
use crate::dag::dag_storage::ItemId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerNodeMap {
    pub(crate) id: ItemId,
    pub(crate) inner: HashMap<PeerId, CertifiedNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerNodeMapMetadata {
    pub(crate) id: ItemId,
    //TODO: either add some fields (like len), or delete this column family.
}

impl PeerNodeMap {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: HashMap::new()
        }
    }

    pub fn get(&self, k: &PeerId) -> Option<&CertifiedNode> {
        self.inner.get(k)
    }

    pub fn insert(&mut self, k: PeerId, v: CertifiedNode) -> Option<CertifiedNode> {
        self.inner.insert(k, v)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<PeerId, CertifiedNode> {
        self.inner.iter()
    }

    pub fn contains_key(&self, k: &PeerId) -> bool {
        self.inner.contains_key(k)
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerNodeMapEntry {
    pub(crate) map_id: ItemId,
    pub(crate) key: Option<PeerId>,
    pub(crate) value_id: Option<HashValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerNodeMapEntry_Key {
    pub(crate) map_id: ItemId,
    pub(crate) maybe_peer_id: Option<PeerId>,
}

impl PeerNodeMapEntry {
    pub(crate) fn key(&self) -> PeerNodeMapEntry_Key {
        PeerNodeMapEntry_Key {
            map_id: self.map_id,
            maybe_peer_id: self.key,
        }
    }
}
