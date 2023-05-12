// Copyright © Aptos Foundation

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
    type WriteBatch;
    fn get_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>>;
    fn new_write_batch(&self) -> Self::WriteBatch;
    fn commit_write_batch(&self, batch: Self::WriteBatch) -> Result<()>;
}

pub(crate) trait DagStoreWriteBatch {
    fn put_dag_in_mem(&mut self, dag_in_mem: &DagInMem) -> Result<()>;
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
}

pub struct NaiveDagStore {
    batch_counter: AtomicUsize,
    write_batches: HashMap<usize, SchemaBatch>,
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
            batch_counter: AtomicUsize::new(0),
            write_batches: HashMap::new(),
            db
        }
    }
}


impl DagStorage for NaiveDagStore {
    type WriteBatch = NaiveDagStoreWriteBatch;

    fn get_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>> {
        //TODO
        Ok(None)
    }

    fn new_write_batch(&self) -> Self::WriteBatch {
        Self::WriteBatch::new()
    }

    fn commit_write_batch(&self, batch: Self::WriteBatch) -> Result<()> {
        self.db.write_schemas(batch.inner)
    }
}



pub struct MockDagStore {}

impl MockDagStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl DagStorage for MockDagStore {
    type WriteBatch = NaiveDagStoreWriteBatch;

    fn get_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>> {
        todo!()
    }

    fn new_write_batch(&self) -> Self::WriteBatch {
        todo!()
    }

    fn commit_write_batch(&self, batch: Self::WriteBatch) -> Result<()> {
        todo!()
    }
}
