// Copyright © Aptos Foundation

use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Write};
use std::sync::Arc;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId, naive};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_item_id};
use crate::dag::types::dag_round_list::{DagRoundListItem, DagRoundListItem_Key};
use aptos_schemadb::define_schema;
use anyhow::Error;
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(DagRoundListItem, DagRoundListItemSchema, "DagRoundListItem");

impl DagStorageItem<NaiveDagStore> for DagRoundListItem {
    type Brief = ItemId;
    type Id = DagRoundListItem_Key;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        let list_id = read_item_id(cursor)?;
        let index = cursor.read_u64::<BigEndian>()?;
        Ok(DagRoundListItem_Key {
            list_id,
            index
        })
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        read_item_id(cursor)
    }

    fn id(&self) -> Self::Id {
        DagRoundListItem_Key {
            list_id: self.list_id,
            index: self.index
        }
    }

    fn brief(&self) -> Self::Brief {
        self.content_id
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        let mut buf = vec![];
        buf.write(id.list_id.as_slice()).unwrap();
        buf.write_u64::<BigEndian>(id.index).unwrap();
        buf
    }

    fn serialize_brief(metadata: &Self::Brief) -> Vec<u8> {
        metadata.to_vec()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(content_id) = db.get::<DagRoundListItemSchema>(id)? {
            Ok(Some(Self{
                list_id: id.list_id,
                index: id.index,
                content_id,
            }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.shallow_save(write_batch)
    }

    impl_default_shallow_ops!(DagRoundListItemSchema);
}
