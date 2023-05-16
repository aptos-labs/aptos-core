// Copyright © Aptos Foundation

use aptos_schemadb::{DB, define_schema, Options, ReadOptions, SchemaBatch};
use std::path::Path;
use std::any::Any;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use anyhow::Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use futures::AsyncWriteExt;
use aptos_consensus_types::node::CertifiedNode;
use aptos_crypto::HashValue;
use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use aptos_types::PeerId;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::dag_storage::naive::certified_node::CertifiedNodeSchema;
use crate::dag::types;
use crate::dag::types::dag_in_mem::{DagInMem, DagInMem_Key, DagInMem_Brief};
use crate::dag::types::dag_round_list::{DagRoundList, DagRoundListItem, DagRoundListItem_Key};
use crate::dag::types::missing_node_status_map::{MissingNodeStatusMap, MissingNodeStatusMapEntry, MissingNodeStatusMapEntry_Key};
use crate::dag::types::peer_index_map::PeerIndexMap;
use crate::dag::types::peer_node_map::{PeerNodeMap, PeerNodeMapEntry, PeerNodeMapEntry_Key, PeerNodeMapMetadata};
use crate::dag::types::peer_status_list::{PeerStatusList, PeerStatusList_Metadata, PeerStatusListItem, PeerStatusListItem_Key};
use crate::dag::types::PeerStatus;
use crate::dag::types::week_link_creator::{WeakLinksCreator, WeakLinksCreator_Brief};

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
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct NaiveDagStore {
    db: DB,
}

impl NaiveDagStore {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            "CertifiedNode",
            "DagInMem",
            "DagRoundList",
            "DagRoundListItem",
            "MissingNodeStatusMapEntry",
            "MissingNodeStatusMap",
            "PeerNodeMap",
            "PeerNodeMapEntry",
            "PeerStatusList",
            "PeerStatusListItem",
            "PeerIndexMap",
            "WeakLinksCreator",
        ];

        let path = db_root_path.as_ref().join(DAG_DB_NAME);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), DAG_DB_NAME, column_families, &opts)
            .expect("DagDB open failed; unable to continue");
        Self {
            db
        }
    }
}

impl DagStorage for NaiveDagStore {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(NaiveDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        let x = batch.as_any().downcast_ref::<NaiveDagStoreWriteBatch>().unwrap();
        self.db.write_schemas_ref(&x.inner)
    }
}

const DAG_DB_NAME: &str = "DagDB";

fn read_bytes(cursor: &mut Cursor<&[u8]>, n: usize) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(n);
    for _ in 0..n {
        let byte = cursor.read_u8()?;
        bytes.push(byte);
    }
    Ok(bytes)
}

/// Implement the default shallow_save and shallow_delete for any implementor of the trait `DatStorageItem<NaiveDagStore>`.
#[macro_export]
macro_rules! impl_default_shallow_ops {
    ($schema:ty) => {
        fn shallow_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
            let batch = write_batch.as_any_mut().downcast_mut::<NaiveDagStoreWriteBatch>().unwrap();
            batch.inner.put::<$schema>(&self.id(), &self.brief())
        }

        fn shallow_delete(id: &Self::Id, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
            let batch = write_batch.as_any_mut().downcast_mut::<NaiveDagStoreWriteBatch>().unwrap();
            batch.inner.delete::<$schema>(id)
        }
    }
}

/// Generate the aptosdb::schema::Schema, KeyCodec and ValueCodec for any implementor of the trait `DatStorageItem<NaiveDagStore>`.
#[macro_export]
macro_rules! define_schema_and_codecs {
    ($typ:ty, $schema:ident, $cf_name:expr) => {
        define_schema!($schema, <$typ as DagStorageItem<NaiveDagStore>>::Id, <$typ as DagStorageItem<NaiveDagStore>>::Brief, $cf_name);

        impl KeyCodec<$schema> for <$typ as DagStorageItem<NaiveDagStore>>::Id {
            fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
                Ok(<$typ as DagStorageItem<NaiveDagStore>>::serialize_id(self))
            }

            fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
                let mut cursor = Cursor::new(data);
                let key = <$typ as DagStorageItem<NaiveDagStore>>::deserialize_id(&mut cursor)?;
                let mut extra_buf = vec![];
                if cursor.read_to_end(&mut extra_buf)? > 0 {
                    Err(Error::msg("Extra bytes."))
                } else {
                    Ok(key)
                }
            }
        }

        impl ValueCodec<$schema> for <$typ as DagStorageItem<NaiveDagStore>>::Brief {
            fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
                Ok(<$typ as DagStorageItem<NaiveDagStore>>::serialize_brief(self))
            }

            fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
                let mut cursor = Cursor::new(data);
                let brief = <$typ as DagStorageItem<NaiveDagStore>>::deserialize_brief(&mut cursor)?;
                let mut extra_buf = vec![];
                if cursor.read_to_end(&mut extra_buf)? > 0 {
                    Err(Error::msg("Extra bytes."))
                } else {
                    Ok(brief)
                }
            }
        }
    }
}


pub(crate) fn read_item_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<ItemId> {
    let buf = read_bytes(cursor, 16)?;
    ItemId::try_from(buf).map_err(|_e|Error::msg("Invalid ItemId serialization."))
}

pub(crate) fn read_peer_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<PeerId> {
    let buf = read_bytes(cursor, 32)?;
    Ok(PeerId::from_bytes(buf)?)
}

pub(crate) fn read_hash_value(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<HashValue> {
    let buf = read_bytes(cursor, 32)?;
    Ok(HashValue::from_slice(buf)?)
}


pub(crate) mod certified_node;
pub(crate) mod peer_node_map_entry;
pub(crate) mod peer_node_map;
pub(crate) mod dag_round_list_item;
pub(crate) mod dag_round_list;
pub(crate) mod peer_status_list_item;
pub(crate) mod peer_status_list;
pub(crate) mod peer_index_map;
pub(crate) mod weak_link_creator;
pub(crate) mod missing_node_status_map_entry;
pub(crate) mod missing_node_status_map;
pub(crate) mod dag_in_mem;
