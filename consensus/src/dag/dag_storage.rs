// Copyright © Aptos Foundation

use std::path::Path;
use aptos_consensus_types::node::CertifiedNode;
use anyhow::Result;
use aptos_schemadb::{DB, Options};
use aptos_types::PeerId;
use crate::dag::dag::DagInMem;

pub(crate) trait DagStorage: Sync + Send {
    fn load_all(&self, epoch: u64) -> Result<Option<DagInMem>>;
    fn save_all(&self, in_mem: &DagInMem) -> Result<()>;
    fn insert_node(&self, round: usize, source: PeerId, node: &CertifiedNode) -> Result<()>;
}

pub struct MockDagStore {}

impl MockDagStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl DagStorage for MockDagStore {
    fn load_all(&self, epoch: u64) -> Result<Option<DagInMem>> {
        todo!()
    }

    fn save_all(&self, in_mem: &DagInMem) -> Result<()> {
        todo!()
    }

    fn insert_node(&self, round: usize, source: PeerId, node: &CertifiedNode) -> Result<()> {
        todo!()
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
            "DagState",
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
    fn load_all(&self, epoch: u64) -> Result<Option<DagInMem>> {
        //TODO
        Ok(None)
    }

    fn save_all(&self, in_mem: &DagInMem) -> Result<()> {
        //TODO
        Ok(())
    }

    fn insert_node(&self, round: usize, source: PeerId, node: &CertifiedNode) -> Result<()> {
        //TODO
        Ok(())
    }
}
