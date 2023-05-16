// Copyright © Aptos Foundation

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::sync::Arc;
use aptos_schemadb::define_schema;
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use aptos_types::PeerId;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_bytes, read_item_id};
use crate::dag::types::peer_index_map::PeerIndexMap;
use std::io::BufRead;

define_schema_and_codecs!(PeerIndexMap, PeerIndexMapSchema, "PeerIndexMap");

impl DagStorageItem<NaiveDagStore> for PeerIndexMap {
    type Brief = HashMap<PeerId, usize>;
    type Id = ItemId;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        read_item_id(cursor)
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        let mut bytes = vec![];
        cursor.read_to_end(&mut bytes)?;
        Ok(bcs::from_bytes(bytes.as_slice())?)
    }

    fn id(&self) -> Self::Id {
        self.id
    }

    fn brief(&self) -> Self::Brief {
        self.inner.clone()
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        id.to_vec()
    }

    fn serialize_brief(brief: &Self::Brief) -> Vec<u8> {
        bcs::to_bytes(brief).unwrap()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(inner) = db.get::<PeerIndexMapSchema>(id)? {
            Ok(Some(PeerIndexMap {
                id: *id,
                inner,
            }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.shallow_save(write_batch)
    }

    impl_default_shallow_ops!(PeerIndexMapSchema);
}
