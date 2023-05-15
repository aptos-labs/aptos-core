// Copyright © Aptos Foundation

use aptos_schemadb::{DB, define_schema, Options, ReadOptions, SchemaBatch};
use std::path::Path;
use std::any::Any;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use anyhow::Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use aptos_consensus_types::node::CertifiedNode;
use aptos_crypto::HashValue;
use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use aptos_types::PeerId;
use crate::dag::dag_storage::{DagStorage, DagStoreWriteBatch, ItemId};
use crate::dag::types;
use crate::dag::types::dag_in_mem::{DagInMem, DagInMem_Key, DagInMem_Metadata};
use crate::dag::types::dag_round_list::{DagRoundList, DagRoundList_Metadata, DagRoundListItem, DagRoundListItem_Key};
use crate::dag::types::missing_node_status_map::{MissingNodeStatusMap, MissingNodeStatusMapEntry, MissingNodeStatusMapEntry_Key};
use crate::dag::types::peer_index_map::PeerIndexMap;
use crate::dag::types::peer_node_map::{PeerNodeMap, PeerNodeMapEntry, PeerNodeMapEntry_Key, PeerNodeMapMetadata};
use crate::dag::types::peer_status_list::{PeerStatusList, PeerStatusList_Metadata, PeerStatusListItem, PeerStatusListItem_Key};
use crate::dag::types::week_link_creator::{WeakLinksCreator, WeakLinksCreatorMetadata};

pub struct NaiveDagStoreWriteBatch {
    inner: SchemaBatch,
}

impl NaiveDagStoreWriteBatch {
    pub(crate) fn new() -> Self {
        Self {
            inner: SchemaBatch::new()
        }
    }
}

impl DagStoreWriteBatch for NaiveDagStoreWriteBatch {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn del_missing_node_id_to_status_map_entry(&mut self, key: &MissingNodeStatusMapEntry_Key) -> anyhow::Result<()> {
        self.inner.delete::<MissingNodeIdToStatusMapEntrySchema>(key)
    }

    fn put_certified_node(&self, obj: &CertifiedNode) -> anyhow::Result<()> {
        self.inner.put::<CertifiedNodeSchema>(&obj.digest(), obj)
    }

    fn put_dag_in_mem__deep(&mut self, obj: &DagInMem) -> anyhow::Result<()> {
        self.put_dag_round_list__shallow(obj.get_dag())?;
        self.put_weak_link_creator__deep(obj.get_front())?;
        self.put_missing_node_id_to_status_map(obj.get_missing_nodes())?;
        self.put_dag_in_mem__shallow(obj)?;
        Ok(())
    }

    fn put_dag_in_mem__shallow(&mut self, obj: &DagInMem) -> anyhow::Result<()> {
        self.inner.put::<DagInMemSchema>(&obj.key(), &obj.metadata())
    }

    fn put_dag_round_list__shallow(&mut self, obj: &DagRoundList) -> anyhow::Result<()> {
        self.inner.put::<DagRoundListSchema>(&obj.id, &obj.metadata())?;
        Ok(())
    }

    fn put_dag_round_list__deep(&mut self, obj: &DagRoundList) -> anyhow::Result<()> {
        for (idx, item) in obj.iter().enumerate() {
            let wrapped_item = DagRoundListItem {
                list_id: obj.id,
                index: idx as u64,
                content_id: item.id,
            };
            self.put_dag_round_list_item(&wrapped_item)?;
        }
        self.put_dag_round_list__shallow(obj)
    }

    fn put_dag_round_list_item(&mut self, obj: &DagRoundListItem) -> anyhow::Result<()> {
        self.inner.put::<DagRoundListItemSchema>(&obj.key(), obj)
    }

    fn put_missing_node_id_to_status_map(&mut self, obj: &MissingNodeStatusMap) -> anyhow::Result<()> {
        self.inner.put::<MissingNodeIdToStatusMapSchema>(&obj.id, obj)
    }

    fn put_missing_node_id_to_status_map_entry(&mut self, obj: &MissingNodeStatusMapEntry) -> anyhow::Result<()> {
        self.inner.put::<MissingNodeIdToStatusMapEntrySchema>(&obj.key(), obj)
    }

    fn put_peer_index_map__deep(&mut self, obj: &PeerIndexMap) -> anyhow::Result<()> {
        self.inner.put::<PeerIndexMapSchema>(&obj.id, obj)
    }

    fn put_peer_status_list__deep(&mut self, obj: &PeerStatusList) -> anyhow::Result<()> {
        for (i, maybe_peer_status) in obj.iter().enumerate() {
            let sub_obj = PeerStatusListItem {
                list_id: obj.id,
                index: i,
                content: maybe_peer_status.clone(),
            };
            self.put_peer_status_list_item(&sub_obj)?;
        }
        self.inner.put::<PeerStatusListSchema>(&obj.id, &obj.metadata())
    }

    fn put_peer_status_list_item(&mut self, obj: &PeerStatusListItem) -> anyhow::Result<()> {
        self.inner.put::<PeerStatusListItemSchema>(&obj.key(), obj)
    }

    fn put_peer_to_node_map__deep(&mut self, obj: &PeerNodeMap) -> anyhow::Result<()> {
        // The entries.
        for (peer, node) in obj.iter() {
            self.put_certified_node(node)?;
            self.put_peer_to_node_map_entry__deep(&PeerNodeMapEntry {
                map_id: obj.id,
                key: *peer,
                value_id: node.digest(),
            })?;
        }

        // The end of the entries.
        self.inner.put::<PeerIdToCertifiedNodeMapEntrySchema>(
            &PeerNodeMapEntry_Key { map_id: obj.id, key: None },
            &None
        )?;

        // The metadata.
        self.inner.put::<PeerIdToCertifiedNodeMapSchema>(&obj.id, &obj.metadata())?;
        Ok(())
    }

    fn put_peer_to_node_map_entry__deep(&mut self, obj: &PeerNodeMapEntry) -> anyhow::Result<()> {
        self.inner.put::<PeerIdToCertifiedNodeMapEntrySchema>(&obj.key(), &Some(obj.clone()))
    }

    fn put_weak_link_creator__deep(&mut self, obj: &WeakLinksCreator) -> anyhow::Result<()> {
        self.put_peer_status_list__deep(&obj.latest_nodes_metadata)?;
        self.put_peer_index_map__deep(&obj.address_to_validator_index)?;
        self.inner.put::<WeakLinksCreatorSchema>(&obj.id, &obj.metadata())?;
        Ok(())
    }
}

pub struct NaiveDagStore {
    db: DB,
}

impl NaiveDagStore {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            "CertifiedNode",
            "DagInMem",
            "DagRoundList",
            "DagRoundListItem",
            "MissingNodeIdToStatusMap",
            "PeerIdToCertifiedNodeMap",
            "PeerIdToCertifiedNodeMapEntry",
            "PeerStatusList",
            "PeerStatusListItem",
            "PeerIndexMap",
            "WeakLinksCreator",
        ];

        let path = db_root_path.as_ref().join(DAG_DB_NAME);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), DAG_DB_NAME, column_families, &opts)
            .expect("ReliableBroadcastDB open failed; unable to continue");
        Self {
            db
        }
    }
}


impl DagStorage for NaiveDagStore {
    fn load_certified_node(&self, key: &HashValue) -> anyhow::Result<Option<CertifiedNode>> {
        self.db.get::<CertifiedNodeSchema>(key)
    }

    fn load_dag_in_mem(&self, key: &DagInMem_Key) -> anyhow::Result<Option<DagInMem>> {
        let maybe_partial = self.db.get::<DagInMemSchema>(key)?;
        if let Some(partial) = maybe_partial {
            let maybe_front = self.load_weak_link_creator(&partial.front)?;
            let maybe_dag =  self.load_dag_round_list(&partial.dag)?;
            let maybe_missing_nodes = self.load_missing_node_id_to_status_map(&partial.missing_nodes)?;
            if let (Some(front), Some(dag), Some(missing_nodes)) = (maybe_front, maybe_dag, maybe_missing_nodes) {
                let obj = DagInMem{
                    my_id: partial.my_id,
                    epoch: partial.epoch,
                    current_round: partial.current_round,
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

    fn load_dag_round_list(&self, key: &ItemId) -> anyhow::Result<Option<DagRoundList>> {
        match self.db.get::<DagRoundListSchema>(key)? {
            Some(metadata) => {
                let mut list = Vec::with_capacity(metadata.len as usize);
                for i in 0..metadata.len {
                    let key = DagRoundListItem_Key { id: metadata.id, index: i };
                    let list_item = self.load_dag_round_list_item(&key)?.unwrap();
                    let map = self.load_peer_to_node_map(&list_item.content_id)?.expect("Inconsistency");
                    list.push(map);
                }
                let obj = DagRoundList {
                    id: metadata.id,
                    inner: list,
                };
                Ok(Some(obj))
            },
            None => {
                Ok(None)
            },
        }
    }

    fn load_dag_round_list_item(&self, key: &DagRoundListItem_Key) -> anyhow::Result<Option<DagRoundListItem>> {
        if let Some(item) = self.db.get::<DagRoundListItemSchema>(key)? {
            Ok(Some(item))
        } else {
            Ok(None)
        }

    }

    fn load_missing_node_id_to_status_map(&self, key: &ItemId) -> anyhow::Result<Option<MissingNodeStatusMap>> {
        if let Some(obj) = self.db.get::<MissingNodeIdToStatusMapSchema>(key)? {
            Ok(Some(obj))
        } else {
            Ok(None)
        }
    }

    fn load_missing_node_id_to_status_map_entry(&self, key: &MissingNodeStatusMapEntry_Key) -> anyhow::Result<Option<MissingNodeStatusMapEntry>> {
        Ok(self.db.get::<MissingNodeIdToStatusMapEntrySchema>(key)?)
    }

    fn load_peer_index_map(&self, key: &ItemId) -> anyhow::Result<Option<PeerIndexMap>> {
        self.db.get::<PeerIndexMapSchema>(key)
    }

    fn load_peer_status_list(&self, key: &ItemId) -> anyhow::Result<Option<PeerStatusList>> {
        if let Some(metadata) = self.db.get::<PeerStatusListSchema>(key)? {
            let list_len = metadata.len as usize;
            let mut list = Vec::with_capacity(list_len);
            for i in 0..list_len {//TODO: parallelize the DB reads?
                let key = PeerStatusListItem_Key { list_id: metadata.id, index: i };
                let list_item = self.load_peer_status_list_item(&key)?.expect("Inconsistency.");
                list.push(list_item.content);
            }
            Ok(Some(PeerStatusList {
                id: metadata.id,
                inner: list,
            }))
        } else {
            Ok(None)
        }

    }

    fn load_peer_status_list_item(&self, key: &PeerStatusListItem_Key) -> anyhow::Result<Option<PeerStatusListItem>> {
        self.db.get::<PeerStatusListItemSchema>(key)
    }

    fn load_peer_to_node_map(&self, key: &ItemId) -> anyhow::Result<Option<PeerNodeMap>> {
        if let Some(metadata) = self.db.get::<PeerIdToCertifiedNodeMapSchema>(key)? {
            let mut iter = self.db.iter::<PeerIdToCertifiedNodeMapEntrySchema>(ReadOptions::default())?;
            iter.seek(&PeerNodeMapEntry_Key {
                map_id: metadata.id,
                key: Some(PeerId::ZERO),
            })?;
            let mut inner = HashMap::new();
            loop {
                let (k, v) = iter.next().unwrap()?;
                if let Some(key) = k.key {
                    let certified_node = self.load_certified_node(&v.unwrap().value_id)?.unwrap();
                    inner.insert(key, certified_node);
                } else {
                    break;
                }
            }
            Ok(Some(PeerNodeMap { id: *key, inner }))
        } else {
            Ok(None)
        }
    }

    fn load_peer_to_node_map_entry(&self, key: &PeerNodeMapEntry_Key) -> anyhow::Result<Option<PeerNodeMapEntry>> {
        Ok(self.db.get::<PeerIdToCertifiedNodeMapEntrySchema>(&key)?.unwrap())
    }

    fn load_weak_link_creator(&self, key: &ItemId) -> anyhow::Result<Option<WeakLinksCreator>> {
        if let Some(obj) = self.db.get::<WeakLinksCreatorSchema>(key)? {
            let maybe_latest_nodes_metadata = self.load_peer_status_list(&obj.latest_nodes_metadata)?;
            let maybe_address_to_validator_index = self.load_peer_index_map(&obj.address_to_validator_index)?;
            if let (Some(latest_nodes_metadata), Some(address_to_validator_index)) = (maybe_latest_nodes_metadata, maybe_address_to_validator_index) {
                let obj = WeakLinksCreator {
                    id: obj.id,
                    my_id: obj.my_id,
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

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(NaiveDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        let x = batch.as_any().downcast_ref::<NaiveDagStoreWriteBatch>().unwrap();
        self.db.write_schemas_ref(&x.inner)
    }
}

const DAG_DB_NAME: &str = "DagDB";

fn read_bytes(cursor: &mut Cursor<&[u8]>, n: usize) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(n);
    for _ in 0..n {
        let byte = cursor.read_u8()?;
        bytes.push(byte);
    }
    Ok(bytes)
}


define_schema!(MissingNodeIdToStatusMapSchema, ItemId, MissingNodeStatusMap, "MissingNodeIdToStatusMap");

define_schema!(MissingNodeIdToStatusMapEntrySchema, MissingNodeStatusMapEntry_Key, MissingNodeStatusMapEntry, "MissingNodeIdToStatusMapEntry");


impl KeyCodec<MissingNodeIdToStatusMapEntrySchema> for MissingNodeStatusMapEntry_Key {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        buf.write(self.map_id.as_slice())?;
        match self.key {
            None => {
                buf.write_u8(0xff)?;
            }
            Some(k) => {
                buf.write_u8(0x00)?;
                buf.write(k.as_slice())?;
            }
        }
        Ok(buf)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = Cursor::new(data);
        let map_id = ItemId::try_from(read_bytes(&mut cursor, 16)?).unwrap();
        let key = match cursor.read_u8()? {
            0x00 => {
                let node_id = HashValue::from_slice(read_bytes(&mut cursor, 32)?.as_slice())?;
                Some(node_id)
            },
            0xff => None,
            _ => unreachable!(),
        };
        Ok(MissingNodeStatusMapEntry_Key {
            map_id,
            key,
        })
    }
}

impl ValueCodec<MissingNodeIdToStatusMapEntrySchema> for MissingNodeStatusMapEntry {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


define_schema!(WeakLinksCreatorSchema, ItemId, WeakLinksCreatorMetadata, "WeakLinksCreator");

define_schema!(DagRoundListSchema, ItemId, DagRoundList_Metadata, "DagRoundList");

define_schema!(DagRoundListItemSchema, DagRoundListItem_Key, DagRoundListItem, "DagRoundListItem");

define_schema!(DagInMemSchema, DagInMem_Key, DagInMem_Metadata, "DagInMem");

define_schema!(PeerStatusListSchema, ItemId, PeerStatusList_Metadata, "PeerStatusList");

define_schema!(PeerStatusListItemSchema, PeerStatusListItem_Key, PeerStatusListItem, "PeerStatusListItem");

define_schema!(PeerIndexMapSchema, ItemId, PeerIndexMap, "PeerIndexMap");

define_schema!(CertifiedNodeSchema, HashValue, CertifiedNode, "CertifiedNode");

impl KeyCodec<CertifiedNodeSchema> for HashValue {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<CertifiedNodeSchema> for CertifiedNode {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


impl KeyCodec<DagInMemSchema> for DagInMem_Key {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<DagInMemSchema> for DagInMem_Metadata {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Write::write(&mut buf, self.my_id.as_slice())?;
        buf.write_u64::<BigEndian>(self.epoch)?;
        buf.write_u64::<BigEndian>(self.current_round)?;
        Write::write(&mut buf, self.front.as_slice())?;
        Write::write(&mut buf, self.dag.as_slice())?;
        Write::write(&mut buf, self.missing_nodes.as_slice())?;
        Ok(buf)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        let mut c = Cursor::new(data);
        let my_id = PeerId::from_bytes(read_bytes(&mut c, 32)?).unwrap();
        let epoch = c.read_u64::<BigEndian>()?;
        let current_round = c.read_u64::<BigEndian>()?;
        let front = ItemId::try_from(read_bytes(&mut c, 16)?).unwrap();
        let dag = ItemId::try_from(read_bytes(&mut c, 16)?).unwrap();
        let missing_nodes = ItemId::try_from(read_bytes(&mut c, 16)?).unwrap();
        let ret = Self {
            my_id,
            epoch,
            current_round,
            front,
            dag,
            missing_nodes,
        };
        Ok(ret)
    }
}


define_schema!(PeerIdToCertifiedNodeMapEntrySchema, PeerNodeMapEntry_Key, Option<PeerNodeMapEntry>, "PeerIdToCertifiedNodeMapEntry");

////////////////////////////////////////////////////////////////////////////////////////

impl KeyCodec<MissingNodeIdToStatusMapSchema> for ItemId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(ItemId::try_from(data)?)
    }
}

impl ValueCodec<MissingNodeIdToStatusMapSchema> for PeerNodeMap {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


impl ValueCodec<PeerIdToCertifiedNodeMapEntrySchema> for Option<PeerNodeMapEntry> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


impl KeyCodec<PeerIdToCertifiedNodeMapEntrySchema> for PeerNodeMapEntry_Key {
    /// Key format: map_id (16 bytes) + [0x00] + key (32 bytes).
    /// In a `*MapEntry` column family, for a map with ID `map_id`, a key `map_id + [0xff]` always exist to help seek.
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        buf.write(self.map_id.as_slice())?;
        match self.key {
            Some(account) => {
                buf.write_u8(0)?;
                buf.write(account.as_slice())?;
            },
            None => {
                buf.write_u8(0xff)?;
            },
        }
        Ok(buf)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = Cursor::new(data);
        let map_id = ItemId::try_from(read_bytes(&mut cursor, 16)?).unwrap();
        match cursor.read_u8()? {
            0 => {
                let key = PeerId::from_bytes(read_bytes(&mut cursor, 32)?).unwrap();
                Ok(Self {
                    map_id,
                    key: Some(key),
                })
            },
            0xff => {
                Ok(Self {
                    map_id,
                    key: None,
                })
            },
            _ => unreachable!()
        }
    }
}

impl KeyCodec<PeerIdToCertifiedNodeMapSchema> for ItemId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(ItemId::try_from(data)?)
    }
}
impl ValueCodec<PeerIdToCertifiedNodeMapSchema> for PeerNodeMapMetadata {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

define_schema!(PeerIdToCertifiedNodeMapSchema, ItemId, PeerNodeMapMetadata, "PeerIdToCertifiedNodeMap");


impl KeyCodec<WeakLinksCreatorSchema> for ItemId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        let x = ItemId::try_from(data)?;
        Ok(x)
    }
}

impl ValueCodec<WeakLinksCreatorSchema> for WeakLinksCreatorMetadata {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        let buf = bcs::to_bytes(self)?;
        Ok(buf)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


impl KeyCodec<PeerIndexMapSchema> for ItemId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(ItemId::try_from(data)?)
    }
}

impl ValueCodec<PeerIndexMapSchema> for PeerIndexMap {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


impl KeyCodec<PeerStatusListSchema> for ItemId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(ItemId::try_from(data)?)
    }
}

impl ValueCodec<PeerStatusListSchema> for PeerStatusList_Metadata {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl KeyCodec<PeerStatusListItemSchema> for PeerStatusListItem_Key {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        buf.write(self.list_id.as_slice())?;
        buf.write_u64::<BigEndian>(self.index as u64)?;
        Ok(buf)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = Cursor::new(data);
        let list_id = ItemId::try_from(read_bytes(&mut cursor, 16)?).unwrap();
        let index = cursor.read_u64::<BigEndian>()? as usize;
        let obj = PeerStatusListItem_Key {
            list_id,
            index,
        };
        Ok(obj)
    }
}

impl ValueCodec<PeerStatusListItemSchema> for PeerStatusListItem {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


impl KeyCodec<DagRoundListItemSchema> for DagRoundListItem_Key {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        buf.write(self.id.as_slice())?;
        buf.write_u64::<BigEndian>(self.index)?;
        Ok(buf)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = Cursor::new(data);
        let id_serialized = read_bytes(&mut cursor, 16)?;
        let id = ItemId::try_from(id_serialized).unwrap();
        let index = cursor.read_u64::<BigEndian>()?;
        Ok(Self {
            id,
            index,
        })
    }
}

impl ValueCodec<DagRoundListItemSchema> for DagRoundListItem {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


impl KeyCodec<DagRoundListSchema> for ItemId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        let obj = ItemId::try_from(data)?;
        Ok(obj)
    }
}

impl ValueCodec<DagRoundListSchema> for DagRoundList_Metadata {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        let buf = bcs::to_bytes(self)?;
        Ok(buf)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        let obj = bcs::from_bytes(data)?;
        Ok(obj)
    }
}

impl ValueCodec<MissingNodeIdToStatusMapSchema> for MissingNodeStatusMap {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        let buf = bcs::to_bytes(self)?;
        Ok(buf)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}
