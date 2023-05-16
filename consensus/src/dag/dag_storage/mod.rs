// Copyright © Aptos Foundation

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use aptos_consensus_types::node::CertifiedNode;
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_schemadb::{DB, Options, SchemaBatch};
use aptos_schemadb::schema::Schema;
use aptos_types::PeerId;
use crate::dag::types::week_link_creator::WeakLinksCreator;
use crate::dag::types::dag_in_mem::{DagInMem, DagInMem_Key};
use crate::dag::types::dag_round_list::{DagRoundList, DagRoundListItem, DagRoundListItem_Key};
use crate::dag::types::missing_node_status_map::{MissingNodeStatusMap, MissingNodeStatusMapEntry, MissingNodeStatusMapEntry_Key};
use crate::dag::types::peer_index_map::PeerIndexMap;
use crate::dag::types::peer_node_map::{PeerNodeMap, PeerNodeMapEntry, PeerNodeMapEntry_Key};
use crate::dag::types::peer_status_list::{PeerStatusList, PeerStatusListItem, PeerStatusListItem_Key};

pub type ItemId = [u8; 16];

pub(crate) trait DagStorageItem<T:DagStorage>: Sized {
    type Brief;
    type Id;
    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> Result<Self::Id>;
    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> Result<Self::Brief>;
    fn id(&self) -> Self::Id;
    fn brief(&self) -> Self::Brief;
    fn serialize_id(id: &Self::Id) -> Vec<u8>;
    fn serialize_brief(brief: &Self::Brief) -> Vec<u8>;
    fn load(store: Arc<dyn DagStorage>, id :&Self::Id) -> Result<Option<Self>>;
    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> Result<()>;
    fn shallow_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> Result<()>;
    fn shallow_delete(id: &Self::Id, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> Result<()>;
    //TODO: deep-deletion is not trivial, but do we need it? If so, how? Reference counting? Pruner?
}

pub(crate) trait DagStorage: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch>;
    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> Result<()>;
}

pub(crate) trait DagStoreWriteBatch: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub(crate) mod naive;
pub(crate) mod mock;
