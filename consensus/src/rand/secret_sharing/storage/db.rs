// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    schema::{SecretShareSchema, SECRET_SHARE_CF_NAME},
    storage_key, LoadedSecretShare, Result, SecretShareStorage, SecretShareStorageError,
};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_schemadb::{batch::SchemaBatch, Options, DB};
use aptos_storage_interface::AptosDbError;
use aptos_types::secret_sharing::SecretShare;
use std::{path::Path, sync::Arc, time::Instant};

pub const SECRET_SHARE_DB_NAME: &str = "secret_share_db";
const MAX_TOTAL_WAL_SIZE_BYTES: u64 = 256 << 20;

pub struct SecretShareDb {
    db: Arc<DB>,
    write_lock: Mutex<()>,
}

impl SecretShareDb {
    pub fn new<P: AsRef<Path>>(db_root_path: P) -> Self {
        let path = db_root_path.as_ref().join(SECRET_SHARE_DB_NAME);
        let instant = Instant::now();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_max_total_wal_size(MAX_TOTAL_WAL_SIZE_BYTES);
        let db = Arc::new(
            DB::open(
                path.clone(),
                SECRET_SHARE_DB_NAME,
                vec![SECRET_SHARE_CF_NAME],
                opts,
            )
            .expect("SecretShareDb open failed; unable to continue"),
        );

        info!(
            "Opened SecretShareDb at {:?} in {} ms",
            path,
            instant.elapsed().as_millis()
        );

        Self {
            db,
            write_lock: Mutex::new(()),
        }
    }

    fn map_db_error(error: AptosDbError) -> SecretShareStorageError {
        match error {
            AptosDbError::BcsError(message)
            | AptosDbError::ParseIntError(message)
            | AptosDbError::Other(message) => SecretShareStorageError::Corruption(message),
            AptosDbError::NotFound(message)
            | AptosDbError::RocksDbIncompleteResult(message)
            | AptosDbError::OtherRocksDbError(message)
            | AptosDbError::IoError(message)
            | AptosDbError::RecvError(message) => SecretShareStorageError::Io(message),
            AptosDbError::TooManyRequested(requested, max) => SecretShareStorageError::Io(format!(
                "too many records requested: {requested}, max {max}"
            )),
            AptosDbError::MissingRootError(version) => {
                SecretShareStorageError::Io(format!("missing root at version {version}"))
            },
            AptosDbError::LedgerPruned {
                data_type,
                version,
                min_available_version,
            } => SecretShareStorageError::Io(format!(
                "{data_type} at version {version} pruned below {min_available_version}"
            )),
            AptosDbError::EventPruned {
                requested_seq_num,
                min_available_seq_num,
            } => SecretShareStorageError::Io(format!(
                "event {requested_seq_num} pruned below {min_available_seq_num}"
            )),
        }
    }
}

impl SecretShareStorage for SecretShareDb {
    fn save_self_share(&self, share: &SecretShare) -> Result<()> {
        let key = storage_key(share.metadata());
        let serialized = bcs::to_bytes(share)
            .map_err(|error| SecretShareStorageError::Corruption(error.to_string()))?;
        let _guard = self.write_lock.lock();

        match self
            .db
            .get::<SecretShareSchema>(&key)
            .map_err(Self::map_db_error)?
        {
            Some(existing) if existing == serialized => Ok(()),
            Some(_) => Err(SecretShareStorageError::Conflict {
                epoch: key.0,
                block_id: key.1,
            }),
            None => {
                let mut batch = SchemaBatch::new();
                batch
                    .put::<SecretShareSchema>(&key, &serialized)
                    .map_err(Self::map_db_error)?;
                self.db.write_schemas(batch).map_err(Self::map_db_error)
            },
        }
    }

    fn load_self_shares(&self, epoch: u64) -> Result<Vec<LoadedSecretShare>> {
        let mut iter = self
            .db
            .iter::<SecretShareSchema>()
            .map_err(Self::map_db_error)?;
        iter.seek_to_first();

        let mut shares = Vec::new();
        for entry in iter {
            let (key, serialized) = match entry.map_err(Self::map_db_error) {
                Ok(entry) => entry,
                Err(error @ SecretShareStorageError::Corruption(_)) => {
                    shares.push(Err(error));
                    continue;
                },
                Err(error) => return Err(error),
            };
            if key.0 != epoch {
                continue;
            }
            let share = bcs::from_bytes::<SecretShare>(&serialized)
                .map_err(|error| SecretShareStorageError::Corruption(error.to_string()))
                .and_then(|share| {
                    if storage_key(share.metadata()) == key {
                        Ok(share)
                    } else {
                        Err(SecretShareStorageError::Corruption(format!(
                            "stored key does not match secret share metadata for epoch {}, block {}",
                            key.0, key.1
                        )))
                    }
                });
            shares.push(share);
        }
        Ok(shares)
    }

    fn prune_before_epoch(&self, epoch: u64) -> Result<()> {
        let _guard = self.write_lock.lock();
        let mut iter = self
            .db
            .iter::<SecretShareSchema>()
            .map_err(Self::map_db_error)?;
        iter.seek_to_first();

        let mut batch = SchemaBatch::new();
        let mut has_deletes = false;
        for entry in iter {
            let (key, _) = entry.map_err(Self::map_db_error)?;
            if key.0 < epoch {
                batch
                    .delete::<SecretShareSchema>(&key)
                    .map_err(Self::map_db_error)?;
                has_deletes = true;
            }
        }
        if has_deletes {
            self.db.write_schemas(batch).map_err(Self::map_db_error)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::secret_sharing::test_utils::{
        create_metadata, create_secret_share, TestContext,
    };
    use aptos_temppath::TempPath;

    #[test]
    fn test_idempotent_write_conflict_and_restart() {
        let temp_path = TempPath::new();
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 10);
        let share = create_secret_share(&ctx, 0, &metadata);

        {
            let db = SecretShareDb::new(&temp_path);
            db.save_self_share(&share).unwrap();
            db.save_self_share(&share).unwrap();

            let mut conflicting = share.clone();
            conflicting.metadata.timestamp += 1;
            assert!(matches!(
                db.save_self_share(&conflicting),
                Err(SecretShareStorageError::Conflict { .. })
            ));
        }

        let reopened = SecretShareDb::new(&temp_path);
        let recovered = reopened
            .load_self_shares(ctx.epoch)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(
            bcs::to_bytes(&recovered).unwrap(),
            bcs::to_bytes(&share).unwrap()
        );
    }

    #[test]
    fn test_prune_before_epoch() {
        let temp_path = TempPath::new();
        let db = SecretShareDb::new(&temp_path);
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let old_metadata = create_metadata(ctx.epoch, 10);
        let new_metadata = create_metadata(ctx.epoch + 1, 11);
        db.save_self_share(&create_secret_share(&ctx, 0, &old_metadata))
            .unwrap();
        db.save_self_share(&create_secret_share(&ctx, 0, &new_metadata))
            .unwrap();

        db.prune_before_epoch(ctx.epoch + 1).unwrap();

        let recovered = db
            .load_self_shares(ctx.epoch + 1)
            .unwrap()
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].metadata(), &new_metadata);
    }

    #[test]
    fn test_corrupt_record_is_rejected() {
        let temp_path = TempPath::new();
        let db = SecretShareDb::new(&temp_path);
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let valid_metadata = create_metadata(ctx.epoch, 10);
        db.save_self_share(&create_secret_share(&ctx, 0, &valid_metadata))
            .unwrap();
        let corrupt_metadata = create_metadata(ctx.epoch, 11);
        let key = storage_key(&corrupt_metadata);
        let mut batch = SchemaBatch::new();
        batch
            .put::<SecretShareSchema>(&key, &vec![0xFF, 0xFF])
            .unwrap();
        db.db.write_schemas(batch).unwrap();

        let records = db.load_self_shares(ctx.epoch).unwrap();
        assert_eq!(records.iter().filter(|record| record.is_ok()).count(), 1);
        assert_eq!(records.iter().filter(|record| record.is_err()).count(), 1);
    }
}
