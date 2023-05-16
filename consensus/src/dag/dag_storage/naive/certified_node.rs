// Copyright © Aptos Foundation

use std::any::Any;
use aptos_consensus_types::node::CertifiedNode;
use aptos_crypto::HashValue;
use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use std::io::{Cursor, Read};
use std::sync::Arc;
use aptos_schemadb::define_schema;
use crate::dag::dag_storage::{DagStorage, DagStorageItem, DagStoreWriteBatch, naive};
use crate::dag::dag_storage::naive::{NaiveDagStore, NaiveDagStoreWriteBatch};
use anyhow::Error;
use std::io::BufRead;

define_schema_and_codecs!(CertifiedNode, CertifiedNodeSchema, "CertifiedNode");

impl DagStorageItem<NaiveDagStore> for CertifiedNode {
    type Brief = Self;
    type Id = HashValue;

    fn deserialize_id(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Id> {
        Ok(HashValue::from_slice(naive::read_bytes(cursor, 32)?)?)
    }

    fn deserialize_brief(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Self::Brief> {
        let mut buf = vec![];
        cursor.read_to_end(&mut buf)?;
        Ok(bcs::from_bytes(buf.as_slice())?)
    }

    fn id(&self) -> Self::Id {
        self.digest()
    }

    fn brief(&self) -> Self::Brief {
        self.clone()
    }

    fn serialize_id(id: &Self::Id) -> Vec<u8> {
        id.to_vec()
    }

    fn serialize_brief(metadata: &Self::Brief) -> Vec<u8> {
        bcs::to_bytes(metadata).unwrap()
    }

    fn load(store: Arc<dyn DagStorage>, id: &Self::Id) -> anyhow::Result<Option<Self>> {
        let db = &store.as_any().downcast_ref::<NaiveDagStore>().unwrap().db;
        db.get::<CertifiedNodeSchema>(id)
    }

    fn deep_save(&self, write_batch: &mut Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        self.shallow_save(write_batch)
    }

    impl_default_shallow_ops!(CertifiedNodeSchema);

}
