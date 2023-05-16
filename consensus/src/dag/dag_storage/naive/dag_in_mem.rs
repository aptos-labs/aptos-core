// Copyright © Aptos Foundation

use std::io::{Cursor, Write};
use std::sync::Arc;
use aptos_schemadb::define_schema;
use aptos_schemadb::schema::KeyCodec;
use aptos_schemadb::schema::ValueCodec;
use anyhow::Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch, read_peer_id};
use crate::dag::types::dag_in_mem::{DagInMem, DagInMem_Brief, DagInMem_Key};
use crate::dag::types::dag_round_list::DagRoundList;
use crate::dag::types::missing_node_status_map::MissingNodeStatusMap;
use crate::dag::types::week_link_creator::WeakLinksCreator;
use std::io::BufRead;
use std::io::Read;

define_schema_and_codecs!(DagInMem, DagInMemSchema, "DagInMem");


impl DagStorageItem<NaiveDagStore> for DagInMem {
    type Brief = DagInMem_Brief;
    type Id = DagInMem_Key;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        let my_id = read_peer_id(cursor)?;
        let epoch = cursor.read_u64::<BigEndian>()?;
        Ok(DagInMem_Key {
            my_id,
            epoch,
        })
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        let mut bytes = vec![];
        cursor.read_to_end(&mut bytes)?;
        Ok(bcs::from_bytes(bytes.as_slice())?)
    }

    fn id(&self) -> Self::Id {
        DagInMem_Key {
            my_id: self.my_id,
            epoch: self.epoch,
        }
    }

    fn brief(&self) -> Self::Brief {
        DagInMem_Brief {
            current_round: self.current_round,
            front: self.front.id,
            dag: self.dag.id,
            missing_nodes: self.missing_nodes.id,
        }
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        let mut buf = vec![];
        buf.write(id.my_id.as_slice()).unwrap();
        buf.write_u64::<BigEndian>(id.epoch).unwrap();
        buf
    }

    fn serialize_brief(brief: &Self::Brief) -> Vec<u8> {
        bcs::to_bytes(brief).unwrap()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        if let Some(brief) = db.get::<DagInMemSchema>(id)? {
            let maybe_front = WeakLinksCreator::load(store.clone(),&brief.front)?;
            let maybe_dag= DagRoundList::load(store.clone(), &brief.dag)?;
            let maybe_missing_nodes = MissingNodeStatusMap::load(store.clone(), &brief.missing_nodes)?;
            if let (Some(front), Some(dag), Some(missing_nodes)) = (maybe_front, maybe_dag, maybe_missing_nodes) {
                let obj = DagInMem {
                    my_id: id.my_id,
                    epoch: id.epoch,
                    current_round: brief.current_round,
                    front,
                    dag,
                    missing_nodes,
                };
                Ok(Some(obj))
            } else {
                Err(Error::msg("Inconsistency."))
            }
        } else {
            Ok(None)
        }
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.dag.deep_save(write_batch)?;
        self.front.deep_save(write_batch)?;
        self.missing_nodes.deep_save(write_batch)?;
        self.shallow_save(write_batch)
    }

    impl_default_shallow_ops!(DagInMemSchema);
}
