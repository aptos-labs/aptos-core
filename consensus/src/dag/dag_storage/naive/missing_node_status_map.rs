// Copyright © Aptos Foundation

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use aptos_schemadb::{define_schema, ReadOptions};
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use byteorder::ReadBytesExt;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_item_id};
use crate::dag::dag_storage::naive::missing_node_status_map_entry::MissingNodeStatusMapEntrySchema;
use crate::dag::types::missing_node_status_map::{MissingNodeStatusMap, MissingNodeStatusMapEntry};
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(MissingNodeStatusMap, MissingNodeStatusMapSchema, "MissingNodeStatusMap");

impl DagStorageItem<NaiveDagStore> for MissingNodeStatusMap {
    type Brief = u8;
    type Id = ItemId;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        read_item_id(cursor)
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        Ok(cursor.read_u8()?)
    }

    fn id(&self) -> Self::Id {
        self.id
    }

    fn brief(&self) -> Self::Brief {
        0
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        id.to_vec()
    }

    fn serialize_brief(brief: &Self::Brief) -> Vec<u8> {
        vec![*brief]
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(brief) = db.get::<MissingNodeStatusMapSchema>(id)? {
            let mut iter = db.iter::<MissingNodeStatusMapEntrySchema>(ReadOptions::default())?;
            let mut inner = HashMap::new();
            loop {
                if let Some(seek_result) = iter.next() {
                    let (db_key, db_value) = seek_result?;
                    match (db_key.key, db_value) {
                        (Some(node_id), Some(missing_node_status)) => {
                            inner.insert(node_id, missing_node_status);
                        },
                        (None, None) => {
                            break;
                        },
                        _ => unreachable!(),
                    }
                } else {
                    unreachable!()
                }
            }
            Ok(Some(MissingNodeStatusMap{ id: *id, inner }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        // The entries.
        for (peer, status) in self.iter() {
            let map_entry = MissingNodeStatusMapEntry {
                map_id: self.id,
                key: Some(*peer),
                value: Some(status.clone()),
            };
            map_entry.deep_save(write_batch)?;
        }

        // The end of the entries.
        let entry_end = MissingNodeStatusMapEntry {
            map_id: self.id,
            key: None,
            value: None,
        };
        entry_end.deep_save(write_batch)?;

        // The metadata.
        self.shallow_save(write_batch)?;
        Ok(())
    }

    impl_default_shallow_ops!(MissingNodeStatusMapSchema);
}
