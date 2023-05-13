// Copyright © Aptos Foundation

use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use aptos_consensus_types::node::CertifiedNode;
use anyhow::Result;
use aptos_schemadb::{DB, Options, SchemaBatch};
use aptos_schemadb::schema::Schema;
use aptos_types::PeerId;
use crate::dag::dag::{DagInMem, DagInMem_Key, DagInMemSchema, PeerIdToCertifiedNodeMap, DagRoundList};

pub type ItemId = [u8; 16];

pub(crate) trait ContainsKey {
    type Key;
    fn key(&self) -> Self::Key;
}

pub(crate) trait DagStorage: Sync + Send {
    fn get_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>>;
    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch>;
    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> Result<()>;
}

pub(crate) trait DagStoreWriteBatch: Sync + Send {
    fn put_dag_in_mem(&mut self, dag_in_mem: &DagInMem) -> Result<()>;
    fn as_any(&self) -> &dyn Any;
}

pub struct NaiveDagStoreWriteBatch {
    inner: SchemaBatch,
}

impl NaiveDagStoreWriteBatch {
    pub(crate) fn new() -> Self {
        Self {
            inner: SchemaBatch::new()
        }
    }
}

impl DagStoreWriteBatch for NaiveDagStoreWriteBatch {
    fn put_dag_in_mem(&mut self, dag_in_mem: &DagInMem) -> Result<()> {
        self.inner.put::<DagInMemSchema>(&dag_in_mem.key(), dag_in_mem)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct NaiveDagStore {
    db: DB,
}

const DAG_DB_NAME: &str = "DagDB";

impl NaiveDagStore {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            "AbsentInfo",
            "DagInMem",
            "DagState.my_id",
            "DagState.epoch",
            "DagState.current_round",
            "DagState.dag",
            "DagState.front",
            "DagState.missing_nodes",
            "PeerIdToCertifiedNodeMap",
            "Map<HashValue,MissingDagNodeStatus>",
            "MissingDagNodeStatus",
            "PendingInfo",
            "PendingInfo.certified_node",
            "PendingInfo.missing_parents",
            "PendingInfo.immediate_dependencies",
            "Set<[u8;32]>",
            "CertifiedNode",
            "NodeMetadata",
            "WeakLinksCreator",
            "WeakLinksCreator.my_id",
            "WeakLinksCreator.latest_nodes_metadata",
            "WeakLinksCreator.address_to_validator_index",
            "Map<PeerId,u64>",
            "Vec<PeerIdToCertifiedNodeMap>",
            "Vec<Option<PeerStatus>>",
            "Option<PeerStatus>",
            "PeerStatus",
            "PeerStatus.case",
            "PeerStatus.caseLinked",
            "PeerStatus.caseNotLinked",
        ];

        let path = db_root_path.as_ref().join(DAG_DB_NAME);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), DAG_DB_NAME, column_families, &opts)
            .expect("ReliableBroadcastDB open failed; unable to continue");
        Self {
            db
        }
    }
}


impl DagStorage for NaiveDagStore {
    fn get_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>> {
        //TODO
        Ok(None)
    }

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(NaiveDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> Result<()> {
        let x = batch.as_any().downcast_ref::<NaiveDagStoreWriteBatch>().unwrap();
        self.db.write_schemas_ref(&x.inner)
    }
}






























pub struct MockDagStoreWriteBatch {}

impl MockDagStoreWriteBatch {
    pub fn new() -> Self {
        Self {}
    }
}

impl DagStoreWriteBatch for MockDagStoreWriteBatch {
    fn put_dag_in_mem(&mut self, dag_in_mem: &DagInMem) -> Result<()> {
        Ok(())
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
    fn get_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>> {
        Ok(None)
    }

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(MockDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> Result<()> {
        Ok(())
    }
}
