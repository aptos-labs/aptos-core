// Copyright © Aptos Foundation

use std::any::Any;
use aptos_consensus_types::node::CertifiedNode;
use aptos_crypto::HashValue;
use crate::dag::dag_storage::{DagStorage, DagStoreWriteBatch, ItemId};
use crate::dag::types::week_link_creator::WeakLinksCreator;
use crate::dag::types::dag_in_mem::{DagInMem, DagInMem_Key};
use crate::dag::types::dag_round_list::{DagRoundList, DagRoundListItem, DagRoundListItem_Key};
use crate::dag::types::missing_node_status_map::{MissingNodeStatusMap, MissingNodeStatusMapEntry, MissingNodeStatusMapEntry_Key};
use crate::dag::types::peer_index_map::PeerIndexMap;
use crate::dag::types::peer_node_map::{PeerNodeMap, PeerNodeMapEntry, PeerNodeMapEntry_Key};
use crate::dag::types::peer_status_list::{PeerStatusList, PeerStatusListItem, PeerStatusListItem_Key};

pub struct MockDagStoreWriteBatch {}

impl MockDagStoreWriteBatch {
    pub fn new() -> Self {
        Self {}
    }
}

impl DagStoreWriteBatch for MockDagStoreWriteBatch {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }
}

pub struct MockDagStore {}

impl MockDagStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl DagStorage for MockDagStore {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(MockDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        Ok(())
    }
}
