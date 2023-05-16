// Copyright © Aptos Foundation

use std::io::{Cursor, Write};
use std::sync::Arc;
use aptos_schemadb::define_schema;
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use byteorder::{ReadBytesExt, WriteBytesExt};
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_bytes, read_hash_value, read_item_id};
use crate::dag::types::missing_node_status_map::{MissingNodeStatusMapEntry, MissingNodeStatusMapEntry_Key};
use crate::dag::types::MissingDagNodeStatus;
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(MissingNodeStatusMapEntry, MissingNodeStatusMapEntrySchema, "MissingNodeStatusMapEntry");

impl DagStorageItem<NaiveDagStore> for MissingNodeStatusMapEntry {
    type Brief = Option<MissingDagNodeStatus>;
    type Id = MissingNodeStatusMapEntry_Key;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        let map_id = read_item_id(cursor)?;
        match cursor.read_u8()? {
            0x00 => {
                let k = read_hash_value(cursor)?;
                Ok(MissingNodeStatusMapEntry_Key {
                    map_id,
                    key: Some(k),
                })
            },
            0xff => Ok(MissingNodeStatusMapEntry_Key {
                map_id,
                key: None,
            }),
            _ => Err(Error::msg("Invariant violated."))
        }
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        let mut bytes = vec![];
        cursor.read_to_end(&mut bytes)?;
        Ok(bcs::from_bytes(bytes.as_slice())?)
    }

    fn id(&self) -> Self::Id {
        MissingNodeStatusMapEntry_Key {
            map_id: self.map_id,
            key: self.key
        }
    }

    fn brief(&self) -> Self::Brief {
        self.value.clone()
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        let mut buf = vec![];
        buf.write(id.map_id.as_slice()).unwrap();
        match id.key {
            Some(k) => {
                buf.write_u8(0x00).unwrap();
                buf.write(k.as_slice()).unwrap();
            },
            None => {
                buf.write_u8(0xff).unwrap();
            }
        }
        buf
    }

    fn serialize_brief(brief: &Self::Brief) -> Vec<u8> {
        bcs::to_bytes(brief).unwrap()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(value) = db.get::<MissingNodeStatusMapEntrySchema>(id)? {
            Ok(Some(MissingNodeStatusMapEntry {
                map_id: id.map_id,
                key: id.key,
                value,
            }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.shallow_save(write_batch)
    }

    impl_default_shallow_ops!(MissingNodeStatusMapEntrySchema);
}
