// Copyright © Aptos Foundation

use std::io::{Cursor, Read, Write};
use std::sync::Arc;
use byteorder::{ReadBytesExt, WriteBytesExt};
use aptos_crypto::HashValue;
use aptos_types::PeerId;
use move_core_types::account_address::AccountAddress;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_bytes, read_item_id};
use crate::dag::types::peer_node_map::{PeerNodeMapEntry, PeerNodeMapEntry_Key};
use aptos_schemadb::define_schema;
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use std::io::BufRead;

define_schema_and_codecs!(PeerNodeMapEntry, PeerNodeMapEntrySchema, "PeerNodeMapEntry");

impl DagStorageItem<NaiveDagStore> for PeerNodeMapEntry {
    type Brief = Option<HashValue>;
    type Id = PeerNodeMapEntry_Key;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        let map_id = read_item_id(cursor)?;
        let key = match cursor.read_u8()? {
            0x00 => Some(PeerId::from_bytes(read_bytes(cursor, 32)?)?),
            0xff => None,
            _ => unreachable!(),
        };
        Ok(PeerNodeMapEntry_Key {
            map_id,
            maybe_peer_id: key,
        })
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        let mut buf = vec![];
        cursor.read_to_end(&mut buf)?;
        Ok(bcs::from_bytes(buf.as_slice())?)
    }

    fn id(&self) -> Self::Id {
        PeerNodeMapEntry_Key {
            map_id: self.map_id,
            maybe_peer_id: self.key,
        }
    }

    fn brief(&self) -> Self::Brief {
        self.value_id
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(id.map_id.as_slice());
        match id.maybe_peer_id {
            None => {
                buf.push(0xff);
            }
            Some(peer_id) => {
                buf.push(0x00);
                buf.extend_from_slice(peer_id.as_slice());
            }
        }
        buf
    }

    fn serialize_brief(brief: &Self::Brief) -> Vec<u8> {
        bcs::to_bytes(brief).unwrap()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(maybe_node_id) = db.get::<PeerNodeMapEntrySchema>(id)? {
            Ok(Some(PeerNodeMapEntry {
                map_id: id.map_id,
                key: id.maybe_peer_id,
                value_id: maybe_node_id,
            }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.shallow_save(write_batch)
    }

    impl_default_shallow_ops!(PeerNodeMapEntrySchema);
}
