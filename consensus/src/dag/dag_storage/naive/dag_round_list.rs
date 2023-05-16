// Copyright © Aptos Foundation

use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use std::io::Cursor;
use std::sync::Arc;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use anyhow::Error;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId, naive};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_item_id};
use crate::dag::types::dag_round_list::{DagRoundList, DagRoundListItem, DagRoundListItem_Key};
use crate::dag::types::peer_node_map::PeerNodeMap;
use aptos_schemadb::define_schema;
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(DagRoundList, DagRoundListSchema, "DagRoundList");

impl DagStorageItem<NaiveDagStore> for DagRoundList {
    type Brief = u64;
    type Id = ItemId;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        read_item_id(cursor)
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        let len = cursor.read_u64::<BigEndian>()?;
        Ok(len)
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
        buf.write_u64::<BigEndian>( *brief).unwrap();
        buf
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(brief) = db.get::<DagRoundListSchema>(id)? {
            let list_len = brief as usize;
            let mut inner = Vec::with_capacity(list_len);
            for i in 0..list_len {
                let list_item =
                    DagRoundListItem::load(store.clone(), &DagRoundListItem_Key{ list_id: *id, index: i as u64 })?
                    .ok_or_else(||Error::msg("Inconsistency"))?;
                let peer_node_map = PeerNodeMap::load(store.clone(), &list_item.content_id)?
                    .ok_or_else(||Error::msg("Inconsistency"))?;
                inner.push(peer_node_map);
            }
            Ok(Some(Self {
                id: *id,
                inner,
            }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        for (i,peer_node_map) in self.inner.iter().enumerate() {
            peer_node_map.deep_save(write_batch)?;
            let list_item = DagRoundListItem{
                list_id: self.id,
                index: i as u64,
                content_id: peer_node_map.id,
            };
            list_item.deep_save(write_batch)?;
        }
        self.shallow_save(write_batch)?;
        Ok(())
    }

    impl_default_shallow_ops!(DagRoundListSchema);
}
