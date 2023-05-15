// Copyright © Aptos Foundation

use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use aptos_consensus_types::node::CertifiedNode;
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_schemadb::{DB, Options, SchemaBatch};
use aptos_schemadb::schema::Schema;
use aptos_types::PeerId;
use crate::dag::types::{DagInMem, DagInMem_Key, DagRoundList, DagRoundListItem, DagRoundListItem_Key, MissingNodeIdToStatusMap, MissingNodeIdToStatusMap_Entry, MissingNodeIdToStatusMap_Entry_Key, PeerIdToCertifiedNodeMap, PeerIdToCertifiedNodeMapEntry, PeerIdToCertifiedNodeMapEntry_Key, PeerIndexMap, PeerStatusList, PeerStatusListItem, PeerStatusListItem_Key, WeakLinksCreator};

pub type ItemId = [u8; 16];

pub fn null_id() -> ItemId {
    [0; 16]
}

pub(crate) trait ContainsKey {
    type Key;
    fn key(&self) -> Self::Key;
}

pub(crate) trait DagStorage: Sync + Send {
    fn load_certified_node(&self, key: &HashValue) -> Result<Option<CertifiedNode>>;
    fn load_dag_in_mem(&self, key: &DagInMem_Key) -> Result<Option<DagInMem>>;
    fn load_dag_round_list(&self, key: &ItemId) -> Result<Option<DagRoundList>>;
    fn load_dag_round_list_item(&self, key: &DagRoundListItem_Key) -> Result<Option<DagRoundListItem>>;
    fn load_missing_node_id_to_status_map(&self, key: &ItemId) -> Result<Option<MissingNodeIdToStatusMap>>;
    fn load_missing_node_id_to_status_map_entry(&self, key: &MissingNodeIdToStatusMap_Entry_Key) -> Result<Option<MissingNodeIdToStatusMap_Entry>>;
    fn load_peer_index_map(&self, key: &ItemId) -> Result<Option<PeerIndexMap>>;
    fn load_peer_status_list(&self, key: &ItemId) -> Result<Option<PeerStatusList>>;
    fn load_peer_status_list_item(&self, key: &PeerStatusListItem_Key) -> Result<Option<PeerStatusListItem>>;
    fn load_peer_to_node_map(&self, key: &ItemId) -> Result<Option<PeerIdToCertifiedNodeMap>>;
    fn load_peer_to_node_map_entry(&self, key: &PeerIdToCertifiedNodeMapEntry_Key) -> Result<Option<PeerIdToCertifiedNodeMapEntry>>;
    fn load_weak_link_creator(&self, key: &ItemId) -> Result<Option<WeakLinksCreator>>;
    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch>;
    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> Result<()>;
}

pub(crate) trait DagStoreWriteBatch: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn del_missing_node_id_to_status_map_entry(&mut self, obj: &MissingNodeIdToStatusMap_Entry_Key) -> Result<()>;
    fn put_certified_node(&self, obj: &CertifiedNode) -> Result<()>;
    fn put_dag_in_mem__deep(&mut self, obj: &DagInMem) -> Result<()>;
    fn put_dag_in_mem__shallow(&mut self, obj: &DagInMem) -> Result<()>;
    fn put_dag_round_list__shallow(&mut self, obj: &DagRoundList) -> Result<()>;
    fn put_dag_round_list__deep(&mut self, obj: &DagRoundList) -> Result<()>;
    fn put_dag_round_list_item(&mut self, obj: &DagRoundListItem) -> Result<()>;
    fn put_missing_node_id_to_status_map(&mut self, obj: &MissingNodeIdToStatusMap) -> Result<()>;
    fn put_missing_node_id_to_status_map_entry(&mut self, obj: &MissingNodeIdToStatusMap_Entry) -> Result<()>;
    fn put_peer_index_map__deep(&mut self, obj: &PeerIndexMap) -> Result<()>;
    fn put_peer_status_list__deep(&mut self, obj: &PeerStatusList) -> Result<()>;
    fn put_peer_status_list_item(&mut self, obj: &PeerStatusListItem) -> Result<()>;
    fn put_peer_to_node_map__deep(&mut self, obj: &PeerIdToCertifiedNodeMap) -> Result<()>;
    fn put_peer_to_node_map_entry__deep(&mut self, obj: &PeerIdToCertifiedNodeMapEntry) -> Result<()>;
    fn put_weak_link_creator__deep(&mut self, obj: &WeakLinksCreator) -> Result<()>;
}

pub(crate) mod naive;
pub(crate) mod mock;
