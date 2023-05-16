// Copyright © Aptos Foundation

use crate::dag::dag_storage::ItemId;
use crate::dag::types::PeerStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusListItem {
    pub(crate) list_id: ItemId,
    pub(crate) index: usize,
    pub(crate) content: Option<PeerStatus>,
}

impl PeerStatusListItem {
    pub(crate) fn key(&self) -> PeerStatusListItem_Key {
        PeerStatusListItem_Key {
            list_id: self.list_id,
            index: self.index,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusListItem_Key {
    pub(crate) list_id: ItemId,
    pub(crate) index: usize,
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusList_Metadata {
    pub(crate) id: ItemId,
    pub(crate) len: u64,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusList {
    pub(crate) id: ItemId,
    pub(crate) inner: Vec<Option<PeerStatus>>,
}

impl PeerStatusList {
    pub(crate) fn new(list: Vec<Option<PeerStatus>>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: list
        }
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<Option<PeerStatus>>{
        self.inner.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> std::slice::IterMut<Option<PeerStatus>>{
        self.inner.iter_mut()
    }

    pub(crate) fn get(&self, i: usize) -> Option<&Option<PeerStatus>> {
        self.inner.get(i)
    }

    pub(crate) fn get_mut(&mut self, i: usize) -> Option<&mut Option<PeerStatus>> {
        self.inner.get_mut(i)
    }
}
