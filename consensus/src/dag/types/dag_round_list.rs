// Copyright © Aptos Foundation

use crate::dag::dag_storage::ItemId;
use crate::dag::types::peer_node_map::PeerNodeMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundList {
    pub(crate) id: ItemId,
    pub(crate) inner: Vec<PeerNodeMap>,
}

impl DagRoundList {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: vec![],
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&PeerNodeMap> {
        self.inner.get(index)
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut PeerNodeMap> {
        self.inner.get_mut(index)
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn push(&mut self, dag_round: PeerNodeMap) {
        self.inner.push(dag_round)
    }

    pub(crate) fn iter(&self) -> core::slice::Iter<PeerNodeMap> {
        self.inner.iter()
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundListItem_Key {
    pub(crate) list_id: ItemId,
    pub(crate) index: u64,
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundListItem {
    pub(crate) list_id: ItemId,
    pub(crate) index: u64,
    pub(crate) content_id: ItemId,
}

impl DagRoundListItem {
    pub(crate) fn key(&self) -> DagRoundListItem_Key {
        DagRoundListItem_Key {
            list_id: self.list_id,
            index: self.index,
        }
    }
}
