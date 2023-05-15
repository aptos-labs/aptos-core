// Copyright © Aptos Foundation

use aptos_schemadb::{DB, Options, SchemaBatch};
use std::path::Path;
use std::any::Any;
use anyhow::Error;
use crate::dag::dag_storage::{ContainsKey, DagStorage, DagStoreWriteBatch, ItemId};
use crate::dag::types::{DagInMem, DagInMem_Key, DagInMemSchema, DagRoundList, DagRoundListItem, DagRoundListItem_Key, DagRoundListItemSchema, DagRoundListSchema, MissingNodeIdToStatusMap, MissingNodeIdToStatusMapSchema, PeerIdToCertifiedNodeMap, PeerIdToCertifiedNodeMapEntry, PeerIdToCertifiedNodeMapEntry_Key, PeerIdToCertifiedNodeMapEntrySchema, PeerIdToCertifiedNodeMapSchema, PeerIndexMap, PeerIndexMapSchema, PeerStatusList, PeerStatusListSchema, WeakLinksCreator, WeakLinksCreatorSchema};

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

    fn put_dag_in_mem__deep(&mut self, obj: &DagInMem) -> anyhow::Result<()> {
        self.inner.put::<DagInMemSchema>(&obj.key(), &obj.partial())?;
        self.put_dag_round_list__shallow(obj.get_dag())?;
        self.put_weak_link_creator__deep(obj.get_front())?;
        self.put_missing_node_id_to_status_map(obj.get_missing_nodes())?;
        Ok(())
    }

    fn put_dag_round_list__shallow(&mut self, obj: &DagRoundList) -> anyhow::Result<()> {
        self.inner.put::<DagRoundListSchema>(&obj.key(), &obj.metadata())?;
        Ok(())
    }

    fn put_dag_round_list_item(&mut self, obj: &DagRoundListItem) -> anyhow::Result<()> {
        self.inner.put::<DagRoundListItemSchema>(&obj.key(), obj)
    }

    fn put_weak_link_creator__deep(&mut self, obj: &WeakLinksCreator) -> anyhow::Result<()> {
        self.put_peer_status_list(&obj.latest_nodes_metadata)?;
        self.put_peer_index_map(&obj.address_to_validator_index)?;
        self.inner.put::<WeakLinksCreatorSchema>(&obj.key(), &obj.metadata())?;
        Ok(())
    }

    fn put_missing_node_id_to_status_map(&mut self, obj: &MissingNodeIdToStatusMap) -> anyhow::Result<()> {
        self.inner.put::<MissingNodeIdToStatusMapSchema>(&obj.key(), obj)
    }

    fn put_peer_to_node_map__deep(&mut self, obj: &PeerIdToCertifiedNodeMap) -> anyhow::Result<()> {
        self.inner.put::<PeerIdToCertifiedNodeMapSchema>(&obj.key(), obj)
    }

    fn put_peer_to_node_map_entry__deep(&mut self, obj: &PeerIdToCertifiedNodeMapEntry) -> anyhow::Result<()> {
        self.inner.put::<PeerIdToCertifiedNodeMapEntrySchema>(&obj.key(), obj)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn put_peer_status_list(&mut self, obj: &PeerStatusList) -> anyhow::Result<()> {
        self.inner.put::<PeerStatusListSchema>(&obj.id, obj)
    }

    fn put_peer_index_map(&mut self, obj: &PeerIndexMap) -> anyhow::Result<()> {
        self.inner.put::<PeerIndexMapSchema>(&obj.id, obj)
    }
}

pub struct NaiveDagStore {
    db: DB,
}

impl NaiveDagStore {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            "DagInMem",
            "DagRoundList",
            "DagRoundListItem",
            "MissingNodeIdToStatusMap",
            "PeerIdToCertifiedNodeMap",
            "PeerIdToCertifiedNodeMapEntry",
            "WeakLinksCreator",
            "PeerStatusList",
            "PeerIndexMap",
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

    fn load_missing_node_id_to_status_map(&self, key: &ItemId) -> anyhow::Result<Option<MissingNodeIdToStatusMap>> {
        if let Some(obj) = self.db.get::<MissingNodeIdToStatusMapSchema>(key)? {
            Ok(Some(obj))
        } else {
            Ok(None)
        }
    }

    fn load_peer_to_node_map(&self, key: &ItemId) -> anyhow::Result<Option<PeerIdToCertifiedNodeMap>> {
        Ok(self.db.get::<PeerIdToCertifiedNodeMapSchema>(key)?)
    }

    fn load_peer_to_node_map_entry(&self, key: &PeerIdToCertifiedNodeMapEntry_Key) -> anyhow::Result<Option<PeerIdToCertifiedNodeMapEntry>> {
        Ok(self.db.get::<PeerIdToCertifiedNodeMapEntrySchema>(&key)?)
    }

    fn load_peer_status_list(&self, key: &ItemId) -> anyhow::Result<Option<PeerStatusList>> {
        self.db.get::<PeerStatusListSchema>(key)
    }

    fn load_peer_index_map(&self, key: &ItemId) -> anyhow::Result<Option<PeerIndexMap>> {
        self.db.get::<PeerIndexMapSchema>(key)
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
