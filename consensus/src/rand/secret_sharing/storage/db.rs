// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    schema::{SecretShareSchema, SECRET_SHARE_CF_NAME},
    storage_key, SecretShareKey, SecretShareStorage,
};
use anyhow::{ensure, Result};
use aptos_logger::info;
use aptos_schemadb::{batch::SchemaBatch, Options, DB};
use aptos_types::secret_sharing::SecretShare;
use std::{path::Path, sync::Arc, time::Instant};

pub const SECRET_SHARE_DB_NAME: &str = "secret_share_db";
const MAX_TOTAL_WAL_SIZE_BYTES: u64 = 256 << 20;

pub struct SecretShareDb {
    db: Arc<DB>,
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

        Self { db }
    }
}

impl SecretShareStorage for SecretShareDb {
    fn save_self_share(&self, share: &SecretShare) -> Result<()> {
        let mut batch = SchemaBatch::new();
        batch.put::<SecretShareSchema>(&storage_key(share.metadata()), share)?;
        self.db.write_schemas(batch)?;
        Ok(())
    }

    fn get_all_self_shares(&self) -> Result<Vec<SecretShare>> {
        let mut iter = self.db.iter::<SecretShareSchema>()?;
        iter.seek_to_first();

        let mut shares = Vec::new();
        for entry in iter {
            let (key, share) = entry?;
            ensure!(
                storage_key(share.metadata()) == key,
                "stored key does not match secret share metadata for epoch {}, block {}",
                key.epoch,
                key.block_id
            );
            shares.push(share);
        }
        Ok(shares)
    }

    fn prune_self_shares(&self, keys: &[SecretShareKey]) -> Result<()> {
        let mut batch = SchemaBatch::new();
        for key in keys {
            batch.delete::<SecretShareSchema>(key)?;
        }
        if !keys.is_empty() {
            self.db.write_schemas_relaxed(batch)?;
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
    fn test_overwrite_and_restart() {
        let temp_path = TempPath::new();
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 10);
        let share = create_secret_share(&ctx, 0, &metadata);

        {
            let db = SecretShareDb::new(&temp_path);
            db.save_self_share(&share).unwrap();
            db.save_self_share(&share).unwrap();

            let mut replacement = share.clone();
            replacement.metadata.timestamp += 1;
            db.save_self_share(&replacement).unwrap();
        }

        let reopened = SecretShareDb::new(&temp_path);
        let recovered = reopened
            .get_all_self_shares()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let mut expected = share;
        expected.metadata.timestamp += 1;
        assert_eq!(
            bcs::to_bytes(&recovered).unwrap(),
            bcs::to_bytes(&expected).unwrap()
        );
    }

    #[test]
    fn test_load_all_self_shares() {
        let temp_path = TempPath::new();
        let db = SecretShareDb::new(&temp_path);
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let old_metadata = create_metadata(ctx.epoch, 10);
        let new_metadata = create_metadata(ctx.epoch + 1, 11);
        db.save_self_share(&create_secret_share(&ctx, 0, &old_metadata))
            .unwrap();
        db.save_self_share(&create_secret_share(&ctx, 0, &new_metadata))
            .unwrap();

        let mut recovered_epochs = db
            .get_all_self_shares()
            .unwrap()
            .into_iter()
            .map(|share| share.epoch())
            .collect::<Vec<_>>();
        recovered_epochs.sort_unstable();
        assert_eq!(recovered_epochs, vec![ctx.epoch, ctx.epoch + 1]);
    }

    #[test]
    fn test_prune_self_shares() {
        let temp_path = TempPath::new();
        let db = SecretShareDb::new(&temp_path);
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = [10, 20, 30].map(|round| create_metadata(ctx.epoch, round));
        for metadata in &metadata {
            db.save_self_share(&create_secret_share(&ctx, 0, metadata))
                .unwrap();
        }

        db.prune_self_shares(&[storage_key(&metadata[0])]).unwrap();

        let mut recovered_rounds = db
            .get_all_self_shares()
            .unwrap()
            .into_iter()
            .map(|share| share.round())
            .collect::<Vec<_>>();
        recovered_rounds.sort_unstable();
        assert_eq!(recovered_rounds, vec![20, 30]);
    }

    #[test]
    fn test_corrupt_record_is_rejected() {
        use aptos_schemadb::batch::WriteBatch;

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
            .raw_put(SECRET_SHARE_CF_NAME, bcs::to_bytes(&key).unwrap(), vec![
                0xFF, 0xFF,
            ])
            .unwrap();
        db.db.write_schemas(batch).unwrap();

        assert!(db.get_all_self_shares().is_err());
    }
}
