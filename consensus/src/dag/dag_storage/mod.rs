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
use crate::dag::dag::{DagInMem, DagInMem_Key, DagInMemSchema, DagRoundList, PeerIdToCertifiedNodeMap};

pub type ItemId = [u8; 16];

pub fn null_id() -> ItemId {
    [0; 16]
}

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

pub(crate) mod naive;
pub(crate) mod mock;

const DAG_DB_NAME: &str = "DagDB";
