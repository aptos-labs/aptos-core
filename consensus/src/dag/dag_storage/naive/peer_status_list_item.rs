// Copyright © Aptos Foundation

use std::io::{Cursor, Read, Write};
use std::sync::Arc;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId, naive};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_item_id};
use crate::dag::types::peer_status_list::{PeerStatusListItem, PeerStatusListItem_Key};
use crate::dag::types::PeerStatus;
use aptos_schemadb::define_schema;
use anyhow::Error;
use std::io::BufRead;

define_schema_and_codecs!(PeerStatusListItem, PeerStatusListItemSchema, "PeerStatusListItem");

impl ValueCodec<PeerStatusListItemSchema> for PeerStatusListItem {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl DagStorageItem<NaiveDagStore> for PeerStatusListItem {
    type Brief = Option<PeerStatus>;
    type Id = PeerStatusListItem_Key;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        let list_id = read_item_id(cursor)?;
        let index = cursor.read_u64::<BigEndian>()? as usize;
        Ok(PeerStatusListItem_Key {
            list_id,
            index,
        })
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        let mut buf = vec![];
        let _ = cursor.read_to_end(&mut buf);
        Ok(bcs::from_bytes(buf.as_slice())?)
    }

    fn id(&self) -> Self::Id {
        PeerStatusListItem_Key {
            list_id: self.list_id,
            index: self.index
        }
    }

    fn brief(&self) -> Self::Brief {
        self.content.clone()
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        let mut buf = vec![];
        buf.write(id.list_id.as_slice()).unwrap();
        buf.write_u64::<BigEndian>(id.index as u64).unwrap();
        buf
    }

    fn serialize_brief(metadata: &Self::Brief) -> Vec<u8> {
        bcs::to_bytes(metadata).unwrap()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(brief) = db.get::<PeerStatusListItemSchema>(id)? {
            Ok(Some(PeerStatusListItem {
                list_id: id.list_id,
                index: id.index,
                content: brief,
            }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.shallow_save(write_batch)
    }

    impl_default_shallow_ops!(PeerStatusListItemSchema);
}
