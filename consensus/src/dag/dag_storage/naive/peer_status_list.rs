// Copyright © Aptos Foundation

use std::io::Cursor;
use std::sync::Arc;
use aptos_schemadb::define_schema;
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_bytes, read_item_id};
use crate::dag::types::peer_status_list::{PeerStatusList, PeerStatusListItem, PeerStatusListItem_Key};
use crate::dag::types::PeerStatus;
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(PeerStatusList, PeerStatusListSchema, "PeerStatusList");

impl DagStorageItem<NaiveDagStore> for PeerStatusList {
    type Brief = u64;
    type Id = ItemId;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        read_item_id(cursor)
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        Ok(cursor.read_u64::<BigEndian>()?)
    }

    fn id(&self) -> Self::Id {
        self.id
    }

    fn brief(&self) -> Self::Brief {
        self.inner.len() as u64
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        id.to_vec()
    }

    fn serialize_brief(brief: &Self::Brief) -> Vec<u8> {
        let mut buf = vec![];
        buf.write_u64::<BigEndian>(*brief).unwrap();
        buf
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(list_len) = db.get::<PeerStatusListSchema>(id)? {
            let list_len = list_len as usize;
            let mut list = Vec::with_capacity(list_len);
            for i in 0..list_len {//TODO: parallelize the DB reads?
                let key = PeerStatusListItem_Key { list_id: *id, index: i };
                let list_item = PeerStatusListItem::load(store.clone(), &key)?.expect("Inconsistency.");
                list.push(list_item.content);
            }
            Ok(Some(PeerStatusList {
                id: *id,
                inner: list,
            }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        for (i, maybe_peer_status) in self.iter().enumerate() {
            let list_item = PeerStatusListItem {
                list_id: self.id,
                index: i,
                content: maybe_peer_status.clone(),
            };
            list_item.deep_save(write_batch)?;
        }
        self.shallow_save(write_batch)?;
        Ok(())
    }

    impl_default_shallow_ops!(PeerStatusListSchema);
}
