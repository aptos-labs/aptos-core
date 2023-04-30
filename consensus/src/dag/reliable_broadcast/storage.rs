// Copyright © Aptos Foundation

use std::path::Path;
use aptos_logger::info;
use aptos_schemadb::{DB, define_schema, Options};
use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use aptos_types::PeerId;
use crate::dag::reliable_broadcast::ReliableBroadcastInMem;
use anyhow::{Error, Result};
use move_core_types::account_address::AccountAddress;

pub const RELIABLE_BROADCAST_DB_NAME: &str = "ReliableBroadcastDB";

pub trait ReliableBroadcastStorage: Sync + Send {
    fn load_all(&self, my_id: PeerId, epoch: u64) -> Option<ReliableBroadcastInMem>; //(peer_id, epoch) enough to identify?
    fn save_all(&self, my_id: PeerId, epoch: u64, in_mem: &ReliableBroadcastInMem);
}

pub struct NaiveReliableBroadcastDB {
    db: DB,
}

#[derive(Debug, PartialEq)]
pub struct NaiveKey {
    id: PeerId,
    epoch: u64,
}

define_schema!(ReliableBroadcastStateSchema, NaiveKey, ReliableBroadcastInMem, "cf1");

impl KeyCodec<ReliableBroadcastStateSchema> for NaiveKey {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(AccountAddress::LENGTH + 8);
        buf.extend_from_slice(self.id.as_slice());
        let epoch_buf = self.epoch.to_be_bytes();
        buf.extend_from_slice(epoch_buf.as_slice());
        Ok(buf)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        if data.len() != AccountAddress::LENGTH + 8 {
            return Err(Error::msg("Invalid key for ReliableBroadcastStateSchema"));
        }
        let id = PeerId::from_bytes(&data[0..AccountAddress::LENGTH])?;
        let mut buf = [0_u8; 8];
        buf.copy_from_slice(&data[AccountAddress::LENGTH..AccountAddress::LENGTH+8]);
        let epoch= u64::from_le_bytes(buf);
        Ok(NaiveKey{ id, epoch })
    }
}

impl ValueCodec<ReliableBroadcastStateSchema> for ReliableBroadcastInMem {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl NaiveReliableBroadcastDB {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec!["cf1"];

        let path = db_root_path.as_ref().join(RELIABLE_BROADCAST_DB_NAME);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), RELIABLE_BROADCAST_DB_NAME, column_families, &opts)
            .expect("ReliableBroadcastDB open failed; unable to continue");

        info!(
            "Opened ReliableBroadcastDB at {:?}",
            path,
        );

        Self { db }
    }
}

impl ReliableBroadcastStorage for NaiveReliableBroadcastDB {
    fn load_all(&self, my_id: PeerId, epoch: u64) -> Option<ReliableBroadcastInMem> {
        info!("NaiveReliableBroadcastDB::load_all(my_id={my_id}, epoch={epoch},in_mem=...)");
        let key = NaiveKey{ id: my_id, epoch };
        self.db.get::<ReliableBroadcastStateSchema>(&key).expect(format!("Failed in loading ReliableBroadcast state for id {my_id}, epoch {epoch} from NaiveReliableBroadcastDB.").as_str())
    }

    fn save_all(&self, my_id: PeerId, epoch: u64, in_mem: &ReliableBroadcastInMem) {
        info!("NaiveReliableBroadcastDB::save_all(my_id={my_id}, epoch={epoch},in_mem=...)");
        let key = NaiveKey{ id: my_id, epoch };
        self.db.put::<ReliableBroadcastStateSchema>(&key, in_mem).expect(format!("Failed in saving ReliableBroadcast state for id {my_id}, epoch {epoch} into NaiveReliableBroadcastDB.").as_str());
    }
}

pub struct MockReliableBroadcastDB {}

impl MockReliableBroadcastDB {
    pub fn new() -> Self {
        Self {}
    }
}

impl ReliableBroadcastStorage for MockReliableBroadcastDB {
    fn load_all(&self, my_id: PeerId, epoch: u64) -> Option<ReliableBroadcastInMem> {
        None
    }

    fn save_all(&self, my_id: PeerId, epoch: u64, in_mem: &ReliableBroadcastInMem) {
    }
}
