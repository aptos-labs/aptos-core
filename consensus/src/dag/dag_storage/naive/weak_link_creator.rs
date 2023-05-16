// Copyright © Aptos Foundation

use std::io::Cursor;
use std::sync::Arc;
use aptos_schemadb::define_schema;
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, ItemId};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_bytes, read_item_id};
use crate::dag::types::peer_index_map::PeerIndexMap;
use crate::dag::types::peer_status_list::PeerStatusList;
use crate::dag::types::week_link_creator::{WeakLinksCreator, WeakLinksCreator_Brief};
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(WeakLinksCreator, WeakLinksCreatorSchema, "WeakLinksCreator");

impl DagStorageItem<NaiveDagStore> for WeakLinksCreator {
    type Brief = WeakLinksCreator_Brief;
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
        WeakLinksCreator_Brief {
            my_id: self.my_id,
            latest_nodes_metadata: self.latest_nodes_metadata.id,
            address_to_validator_index: self.address_to_validator_index.id,
        }
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        id.to_vec()
    }

    fn serialize_brief(brief: &Self::Brief) -> Vec<u8> {
        bcs::to_bytes(brief).unwrap()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(brief) = db.get::<WeakLinksCreatorSchema>(id)? {
            let maybe_latest_nodes_metadata = PeerStatusList::load(store.clone(), &brief.latest_nodes_metadata)?;
            let maybe_address_to_validator_index = PeerIndexMap::load(store.clone(), &brief.address_to_validator_index)?;
            if let (Some(latest_nodes_metadata), Some(address_to_validator_index)) = (maybe_latest_nodes_metadata, maybe_address_to_validator_index) {
                let obj = WeakLinksCreator {
                    id: *id,
                    my_id: brief.my_id,
                    latest_nodes_metadata,
                    address_to_validator_index,
                };
                Ok(Some(obj))
            } else {
                Err(Error::msg("Inconsistency"))
            }
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.address_to_validator_index.deep_save(write_batch)?;
        self.latest_nodes_metadata.deep_save(write_batch)?;
        self.shallow_save(write_batch)?;
        Ok(())
    }

    impl_default_shallow_ops!(WeakLinksCreatorSchema);
}
