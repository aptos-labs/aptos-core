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
use crate::dag::types::{DagInMem, DagInMem_Key, DagRoundList, DagRoundListItem, DagRoundListItem_Key, MissingNodeIdToStatusMap, PeerIdToCertifiedNodeMap, WeakLinksCreator};

pub type ItemId = [u8; 16];

pub fn null_id() -> ItemId {
    [0; 16]
}

pub(crate) trait ContainsKey {
    type Key;
    fn key(&self) -> Self::Key;
}

pub(crate) trait DagStorage: Sync + Send {
    fn load_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>>;
    fn load_weak_link_creator(&self, key: &ItemId) -> Result<Option<WeakLinksCreator>>;
    fn load_dag_round_list(&self, key: &ItemId) -> Result<Option<DagRoundList>>;
    fn load_dag_round_list_item(&self, key: &DagRoundListItem_Key) -> Result<Option<DagRoundListItem>>;
    fn load_missing_node_id_to_status_map(&self, key: &ItemId) -> Result<Option<MissingNodeIdToStatusMap>>;
    fn load_peer_to_node_map(&self, key: &ItemId) -> Result<Option<PeerIdToCertifiedNodeMap>>;
    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch>;
    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> Result<()>;
}

pub(crate) trait DagStoreWriteBatch: Sync + Send {
    fn put_dag_in_mem(&mut self, obj: &DagInMem) -> Result<()>;
    fn put_dag_round_list(&mut self, obj: &DagRoundList) -> Result<()>;
    fn put_dag_round_list_item(&mut self, obj: &DagRoundListItem) -> Result<()>;
    fn put_weak_link_creator(&mut self, obj: &WeakLinksCreator) -> Result<()>;
    fn put_missing_node_id_to_status_map(&mut self, obj: &MissingNodeIdToStatusMap) -> Result<()>;
    fn put_peer_to_node_map(&mut self, obj: &PeerIdToCertifiedNodeMap) -> Result<()>;

    fn as_any(&self) -> &dyn Any;
}

pub(crate) mod naive;
pub(crate) mod mock;
