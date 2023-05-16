// Copyright © Aptos Foundation

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use byteorder::ReadBytesExt;
use aptos_consensus_types::node::CertifiedNode;
use aptos_schemadb::ReadOptions;
use aptos_types::PeerId;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_bytes, read_item_id};
use crate::dag::types::peer_node_map::{PeerNodeMap, PeerNodeMapEntry, PeerNodeMapEntry_Key};
use aptos_schemadb::define_schema;
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use crate::dag::dag_storage::naive::peer_node_map_entry::PeerNodeMapEntrySchema;
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(PeerNodeMap, PeerNodeMapSchema, "PeerNodeMap");

impl DagStorageItem<NaiveDagStore> for PeerNodeMap {
    type Brief = u8;
    type Id = ItemId;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        let id = read_item_id(cursor)?;
        Ok(id)
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        if cursor.read_u8()? == 0 {
            Ok(0)
        } else {
            Err(Error::msg("Invalid brief."))
        }
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

    fn serialize_brief(_brief: &Self::Brief) -> Vec<u8> {
        vec![0]
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(brief) = db.get::<PeerNodeMapSchema>(id)? {
            let mut iter = db.iter::<PeerNodeMapEntrySchema>(ReadOptions::default())?;
            let start_key = PeerNodeMapEntry_Key {
                map_id: *id,
                maybe_peer_id: Some(PeerId::ZERO),
            };
            iter.seek(&start_key)?;
            let mut inner = HashMap::new();
            loop { //TODO: multiple sequential loads here... Async?
                let (key, maybe_node) = iter.next().unwrap()?;
                match (key.maybe_peer_id, maybe_node) {
                    (Some(peer_id), Some(node_id)) => {
                        let certified_node = CertifiedNode::load(store.clone(), &node_id)?.ok_or_else(||Error::msg("Inconsistency detected."))?;
                        inner.insert(peer_id, certified_node);
                    },
                    (None,None) => {
                        break;
                    },
                    _ => unreachable!(),
                }
            }
            Ok(Some(PeerNodeMap { id: *id, inner }))
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        // The entries.
        for (peer, node) in self.iter() {
            node.deep_save(write_batch)?;
            let map_entry = PeerNodeMapEntry {
                map_id: self.id,
                key: Some(*peer),
                value_id: Some(node.digest()),
            };
            map_entry.deep_save(write_batch)?;
        }

        // The end of the entries.
        let entry_end = PeerNodeMapEntry {
            map_id: self.id,
            key: None,
            value_id: None,
        };
        entry_end.deep_save(write_batch)?;

        // The metadata.
        self.shallow_save(write_batch)?;
        Ok(())
    }

    impl_default_shallow_ops!(PeerNodeMapSchema);
}
