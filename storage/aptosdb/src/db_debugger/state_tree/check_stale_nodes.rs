// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db_debugger::common::DbDir,
    schema::{
        db_metadata::DbMetadataKey, stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    },
    utils::get_progress,
};
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_schemadb::{schema::Schema, DB};
use aptos_storage_interface::Result;
use clap::Parser;
use std::collections::BTreeMap;

const DEFAULT_LIMIT_PER_DB: usize = 20;

#[derive(Parser)]
#[clap(about = "Check for stale JMT nodes that should have been pruned.")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    #[clap(long, default_value_t = DEFAULT_LIMIT_PER_DB)]
    limit_per_db: usize,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let state_merkle_db = self.db_dir.open_state_merkle_db()?;

        // Step 1: Read pruner progress.
        let p1 = get_progress(
            state_merkle_db.metadata_db(),
            &DbMetadataKey::StateMerklePrunerProgress,
        )?
        .unwrap_or(0);
        let p2 = get_progress(
            state_merkle_db.metadata_db(),
            &DbMetadataKey::EpochEndingStateMerklePrunerProgress,
        )?
        .unwrap_or(0);

        println!("State Merkle Pruner progress: {p1}");
        println!("Epoch Ending State Merkle Pruner progress: {p2}");

        // Step 2: Collect DB handles.
        let num_shards = state_merkle_db.num_shards();
        let mut dbs: Vec<(String, &DB)> =
            vec![("metadata".to_string(), state_merkle_db.metadata_db())];
        for shard_id in 0..num_shards {
            dbs.push((
                format!("shard_{shard_id}"),
                state_merkle_db.db_shard(shard_id),
            ));
        }

        // Step 3: Collect leaked stale node indices per DB.
        let mut regular_leaked: BTreeMap<String, Vec<StaleNodeIndex>> = BTreeMap::new();
        let mut cross_epoch_leaked: BTreeMap<String, Vec<StaleNodeIndex>> = BTreeMap::new();

        for (db_name, db) in &dbs {
            println!("Scanning {db_name} for stale node indices...");

            let regular = collect_stale_indices::<StaleNodeIndexSchema>(db, p1, self.limit_per_db)?;
            let cross_epoch =
                collect_stale_indices::<StaleNodeIndexCrossEpochSchema>(db, p2, self.limit_per_db)?;

            println!(
                "Done scanning {db_name}. regular: {}, cross_epoch: {}",
                regular.len(),
                cross_epoch.len(),
            );

            if !regular.is_empty() {
                regular_leaked.insert(db_name.clone(), regular);
            }
            if !cross_epoch.is_empty() {
                cross_epoch_leaked.insert(db_name.clone(), cross_epoch);
            }
        }

        // Step 4: Report.
        let regular_total: usize = regular_leaked.values().map(|v| v.len()).sum();
        println!(
            "\nLeaked regular stale node indices (stale_since_version <= {p1}): {regular_total}"
        );
        for (db_name, indices) in &regular_leaked {
            println!("  {db_name}: {}", indices.len());
            for index in indices {
                println!(
                    "    stale_since_version: {}, node_key: {:?}",
                    index.stale_since_version, index.node_key,
                );
            }
        }

        let cross_epoch_total: usize = cross_epoch_leaked.values().map(|v| v.len()).sum();
        println!(
            "\nLeaked cross-epoch stale node indices (stale_since_version <= {p2}): {cross_epoch_total}"
        );
        for (db_name, indices) in &cross_epoch_leaked {
            println!("  {db_name}: {}", indices.len());
            for index in indices {
                println!(
                    "    stale_since_version: {}, node_key: {:?}",
                    index.stale_since_version, index.node_key,
                );
            }
        }

        println!("\nTotal leaked: {}", regular_total + cross_epoch_total);

        Ok(())
    }
}

/// Collects stale node index entries with `stale_since_version <= progress` in the given DB,
/// returning at most `limit` entries.
fn collect_stale_indices<S>(db: &DB, progress: u64, limit: usize) -> Result<Vec<StaleNodeIndex>>
where
    S: Schema<Key = StaleNodeIndex, Value = ()>,
    StaleNodeIndex: aptos_schemadb::schema::KeyCodec<S>,
{
    let mut result = Vec::new();
    let mut iter = db.iter::<S>()?;
    iter.seek_to_first();
    for item in iter {
        let (index, _) = item?;
        if index.stale_since_version > progress {
            break;
        }
        result.push(index);
        if result.len() >= limit {
            break;
        }
    }
    Ok(result)
}
