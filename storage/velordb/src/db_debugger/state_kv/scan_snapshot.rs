// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_debugger::common::DbDir,
    schema::{state_value::StateValueSchema, state_value_by_key_hash::StateValueByKeyHashSchema},
};
use velor_crypto::hash::CryptoHash;
use velor_jellyfish_merkle::iterator::JellyfishMerkleIterator;
use velor_schemadb::ReadOptions;
use velor_storage_interface::Result;
use velor_types::transaction::Version;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::{
    sync::{mpsc, Arc},
    thread,
    time::Instant,
};

#[derive(Parser)]
#[clap(about = "Print state value.")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    #[clap(long)]
    version: Version,

    #[clap(long, default_value = "32")]
    concurrency: usize,

    #[clap(long, default_value = "100")]
    slow_threshold_ms: u128,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        println!(
            "{}",
            format!(
                "* Scan all key values in snapshot at version {} in the key hash value order. \n",
                self.version,
            )
            .yellow()
        );

        let state_kv_db = Arc::new(self.db_dir.open_state_kv_db()?);
        let state_merkle_db = Arc::new(self.db_dir.open_state_merkle_db()?);
        let total_leaves = state_merkle_db.get_leaf_count(self.version)?;
        println!("total leaves: {}", total_leaves);

        let (range_tx, range_rx) = crossbeam_channel::bounded::<(usize, usize)>(1024);
        let (result_tx, result_rx) = mpsc::channel();

        let workers: Vec<_> = (0..self.concurrency)
            .map(|_| {
                let range_rx = range_rx.clone();
                let state_merkle_db = state_merkle_db.clone();
                let state_kv_db = state_kv_db.clone();
                let result_tx = result_tx.clone();
                thread::spawn(move || {
                    while let Ok((start, len)) = range_rx.recv() {
                        let range_iter = JellyfishMerkleIterator::new_by_index(
                            state_merkle_db.clone(),
                            self.version,
                            start,
                        )
                        .unwrap()
                        .take(len);

                        for (n, leaf_res) in range_iter.enumerate() {
                            let (_key_hash, (key, key_version)) = leaf_res.unwrap();
                            let index = start + n;

                            let t = Instant::now();

                            let mut read_opts = ReadOptions::default();
                            // We want `None` if the state_key changes in iteration.
                            read_opts.set_prefix_same_as_start(true);

                            let enable_sharding = state_kv_db.enabled_sharding();

                            let (value_version, value) = if enable_sharding {
                                let mut iter = state_kv_db
                                    .db_shard(key.get_shard_id())
                                    .iter::<StateValueByKeyHashSchema>()
                                    .unwrap();
                                iter.seek(&(key.hash(), key_version)).unwrap();
                                iter.next()
                                    .transpose()
                                    .unwrap()
                                    .and_then(|((_, version), value_opt)| {
                                        value_opt.map(|value| (version, value))
                                    })
                                    .expect("Value must exist.")
                            } else {
                                let mut iter = state_kv_db
                                    .db_shard(key.get_shard_id())
                                    .iter::<StateValueSchema>()
                                    .unwrap();
                                iter.seek(&(key.clone(), key_version)).unwrap();
                                iter.next()
                                    .transpose()
                                    .unwrap()
                                    .and_then(|((_, version), value_opt)| {
                                        value_opt.map(|value| (version, value))
                                    })
                                    .expect("Value must exist.")
                            };

                            let elapsed = t.elapsed();
                            result_tx
                                .send((index, key, key_version, value_version, value, elapsed))
                                .unwrap();
                        }
                    }
                })
            })
            .collect();

        let printer = thread::spawn(move || {
            let start_time = Instant::now();
            let bar = ProgressBar::new(total_leaves as u64);
            bar.set_style(ProgressStyle::default_bar().template(
                "[{elapsed_precise} {per_sec}] {bar:100.cyan/blue} {pos} / {len} {percent}% ETA {eta_precise}",
            ));

            for count in 0..total_leaves {
                let (index, key, key_version, value_version, value, elapsed) =
                    result_rx.recv().unwrap();
                bar.inc(1);

                if count % 1000000 == 0 {
                    bar.println(format!(
                        "{count} leaves scanned, total time: {:?}",
                        start_time.elapsed()
                    ));
                }
                if elapsed.as_millis() > self.slow_threshold_ms {
                    let serialized = hex::encode(bcs::to_bytes(&key).unwrap());

                    println!("{}", "- Slow fetch detected!".to_string().red());
                    println!("         time: {:?}", elapsed);
                    println!("   leaf index: {}", index);
                    println!("    state key: {:?}\n", key);
                    println!("   serialized: {}\n", serialized);
                    println!(" leaf version: {}", key_version);
                    println!("        Value: ");
                    println!("            version: {value_version}");
                    print!("              bytes: ({} bytes)", value.bytes().len());
                    if value.bytes().len() > 1024 {
                        println!();
                    } else {
                        println!("{:?}", value.bytes());
                    }
                    println!("           metadata: {:?}", value.into_metadata());
                    println!(); // extra blank line, otherwise the progress bar overwrites last line.
                }
            }

            bar.finish()
        });

        const BATCH_SIZE: usize = 100_000;
        let mut start = 0;
        while start <= total_leaves {
            if total_leaves - start < BATCH_SIZE {
                range_tx.send((start, total_leaves - start)).unwrap();
                break;
            } else {
                range_tx.send((start, BATCH_SIZE)).unwrap();
                start += BATCH_SIZE;
            }
        }

        // signal workers to quit
        drop(range_tx);

        // wait for work to drain
        printer.join().unwrap();
        for worker in workers {
            worker.join().unwrap();
        }

        println!("{}", "Scan complete.".to_string().yellow());

        Ok(())
    }
}
