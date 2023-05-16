// Copyright © Aptos Foundation

use std::collections::HashMap;
use move_core_types::account_address::AccountAddress as PeerId;
use crate::dag::dag_storage::ItemId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerIndexMap {
    pub(crate) id: ItemId,
    pub(crate) inner: HashMap<PeerId, usize>,
}

impl PeerIndexMap {
    pub(crate) fn new(inner: HashMap<PeerId, usize>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner,
        }
    }

    pub(crate) fn get(&self, k: &PeerId) -> Option<&usize> {
        self.inner.get(k)
    }
}
