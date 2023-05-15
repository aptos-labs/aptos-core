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
    fn del_missing_node_id_to_status_map_entry(&mut self, obj: &MissingNodeStatusMapEntry_Key) -> anyhow::Result<()> {
        todo!()
    }

    fn put_certified_node(&self, obj: &CertifiedNode) -> anyhow::Result<()> {
        todo!()
    }

    fn put_dag_in_mem__deep(&mut self, dag_in_mem: &DagInMem) -> anyhow::Result<()> {
        Ok(())
    }

    fn put_dag_in_mem__shallow(&mut self, obj: &DagInMem) -> anyhow::Result<()> {
        todo!()
    }

    fn put_dag_round_list__shallow(&mut self, dag_round_list: &DagRoundList) -> anyhow::Result<()> {
        todo!()
    }

    fn put_dag_round_list__deep(&mut self, obj: &DagRoundList) -> anyhow::Result<()> {
        todo!()
    }

    fn put_dag_round_list_item(&mut self, obj: &DagRoundListItem) -> anyhow::Result<()> {
        todo!()
    }

    fn put_weak_link_creator__deep(&mut self, obj: &WeakLinksCreator) -> anyhow::Result<()> {
        todo!()
    }

    fn put_missing_node_id_to_status_map(&mut self, obj: &MissingNodeStatusMap) -> anyhow::Result<()> {
        todo!()
    }

    fn put_missing_node_id_to_status_map_entry(&mut self, obj: &MissingNodeStatusMapEntry) -> anyhow::Result<()> {
        todo!()
    }

    fn put_peer_to_node_map__deep(&mut self, obj: &PeerNodeMap) -> anyhow::Result<()> {
        todo!()
    }

    fn put_peer_to_node_map_entry__deep(&mut self, obj: &PeerNodeMapEntry) -> anyhow::Result<()> {
        todo!()
    }

    fn put_peer_status_list__deep(&mut self, obj: &PeerStatusList) -> anyhow::Result<()> {
        todo!()
    }

    fn put_peer_status_list_item(&mut self, obj: &PeerStatusListItem) -> anyhow::Result<()> {
        todo!()
    }

    fn put_peer_index_map__deep(&mut self, obj: &PeerIndexMap) -> anyhow::Result<()> {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct MockDagStore {}

impl MockDagStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl DagStorage for MockDagStore {
    fn load_certified_node(&self, key: &HashValue) -> anyhow::Result<Option<CertifiedNode>> {
        todo!()
    }

    fn load_dag_in_mem(&self, key: &DagInMem_Key) -> anyhow::Result<Option<DagInMem>> {
        Ok(None)
    }

    fn load_weak_link_creator(&self, key: &ItemId) -> anyhow::Result<Option<WeakLinksCreator>> {
        todo!()
    }

    fn load_dag_round_list(&self, key: &ItemId) -> anyhow::Result<Option<DagRoundList>> {
        todo!()
    }

    fn load_dag_round_list_item(&self, key: &DagRoundListItem_Key) -> anyhow::Result<Option<DagRoundListItem>> {
        todo!()
    }

    fn load_missing_node_id_to_status_map(&self, key: &ItemId) -> anyhow::Result<Option<MissingNodeStatusMap>> {
        todo!()
    }

    fn load_missing_node_id_to_status_map_entry(&self, key: &MissingNodeStatusMapEntry_Key) -> anyhow::Result<Option<MissingNodeStatusMapEntry>> {
        todo!()
    }

    fn load_peer_to_node_map(&self, key: &ItemId) -> anyhow::Result<Option<PeerNodeMap>> {
        todo!()
    }

    fn load_peer_to_node_map_entry(&self, key: &PeerNodeMapEntry_Key) -> anyhow::Result<Option<PeerNodeMapEntry>> {
        todo!()
    }

    fn load_peer_status_list(&self, key: &ItemId) -> anyhow::Result<Option<PeerStatusList>> {
        todo!()
    }

    fn load_peer_status_list_item(&self, key: &PeerStatusListItem_Key) -> anyhow::Result<Option<PeerStatusListItem>> {
        todo!()
    }

    fn load_peer_index_map(&self, key: &ItemId) -> anyhow::Result<Option<PeerIndexMap>> {
        todo!()
    }

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(MockDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        Ok(())
    }
}
