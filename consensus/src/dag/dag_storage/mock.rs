// Copyright © Aptos Foundation

use std::any::Any;
use crate::dag::dag::{DagInMem, DagInMem_Key, DagRoundList, WeakLinksCreator};
use crate::dag::dag_storage::{DagStorage, DagStoreWriteBatch, ItemId};
use crate::dag::types::MissingNodeIdToStatusMap;

pub struct MockDagStoreWriteBatch {}

impl MockDagStoreWriteBatch {
    pub fn new() -> Self {
        Self {}
    }
}

impl DagStoreWriteBatch for MockDagStoreWriteBatch {
    fn put_dag_in_mem(&mut self, dag_in_mem: &DagInMem) -> anyhow::Result<()> {
        Ok(())
    }

    fn put_dag_round_list(&mut self, dag_round_list: &DagRoundList) -> anyhow::Result<()> {
        todo!()
    }

    fn put_weak_link_creator(&mut self, obj: &WeakLinksCreator) -> anyhow::Result<()> {
        todo!()
    }

    fn put_missing_node_id_to_status_map(&mut self, obj: &MissingNodeIdToStatusMap) -> anyhow::Result<()> {
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
    fn load_dag_in_mem(&self, key: &DagInMem_Key) -> anyhow::Result<Option<DagInMem>> {
        Ok(None)
    }

    fn load_weak_link_creator(&self, key: &ItemId) -> anyhow::Result<Option<WeakLinksCreator>> {
        todo!()
    }

    fn load_dag_round_list(&self, key: &ItemId) -> anyhow::Result<Option<DagRoundList>> {
        todo!()
    }

    fn load_missing_node_id_to_status_map(&self, key: &ItemId) -> anyhow::Result<Option<MissingNodeIdToStatusMap>> {
        todo!()
    }

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(MockDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        Ok(())
    }
}
