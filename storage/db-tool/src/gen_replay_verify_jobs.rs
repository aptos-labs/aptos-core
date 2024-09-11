// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_backup_cli::{
    metadata::{
        cache::{sync_and_load, MetadataCacheOpt},
        StateSnapshotBackupMeta,
    },
    storage::DBToolStorageOpt,
    utils::ConcurrentDownloadsOpt,
};
use aptos_logger::info;
use aptos_types::transaction::Version;
use clap::Parser;
use itertools::{zip_eq, Itertools};
use std::{
    io::Write,
    iter::{once, zip},
    path::PathBuf,
};

#[derive(Parser)]
pub struct Opt {
    #[clap(flatten)]
    metadata_cache_opt: MetadataCacheOpt,
    #[clap(flatten)]
    storage: DBToolStorageOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(
        long,
        help = "The first transaction version required to be replayed and verified. [Defaults to 0]"
    )]
    start_version: Option<Version>,
    #[clap(
        long,
        help = "Target number of transactions for each job to replay",
        default_value = "1500000"
    )]
    target_job_size: u64,
    #[clap(
        long,
        help = "Determines the oldest epoch to replay, relative to the latest",
        default_value = "4000"
    )]
    max_epochs: u64,
    #[clap(
        long,
        help = "Version ranges to skip. e.g. 123-2456",
        value_delimiter = ' '
    )]
    ranges_to_skip: Vec<String>,
    #[clap(long, help = "Output job ranges to json files, evenly distributed.")]
    output_json_files: Vec<PathBuf>,
}

impl Opt {
    pub async fn run(self) -> anyhow::Result<()> {
        assert!(!self.output_json_files.is_empty());

        let storage = self.storage.init_storage().await?;
        let metadata_view = sync_and_load(
            &self.metadata_cache_opt,
            storage,
            self.concurrent_downloads.get(),
        )
        .await?;

        let storage_state = metadata_view.get_storage_state()?;
        let global_end_version = storage_state
            .latest_transaction_version
            .expect("No transaction backups.")
            + 1;
        let latest_epoch = storage_state
            .latest_state_snapshot_epoch
            .expect("No state snapshots.");
        let max_epochs = self.max_epochs.min(latest_epoch + 1);
        let global_min_epoch = latest_epoch + 1 - max_epochs;

        let fake_end = StateSnapshotBackupMeta {
            epoch: latest_epoch,
            version: global_end_version,
            manifest: "".to_string(),
        };
        let job_ranges = metadata_view
            .all_state_snapshots()
            .iter()
            .dedup_by(|a, b| a.epoch == b.epoch)
            .filter(|s| s.epoch >= global_min_epoch && s.version <= global_end_version)
            .chain(once(&fake_end))
            .collect_vec()
            .iter()
            .rev()
            .tuple_windows()
            // to simplify things, if start_version appears in the middle of a range, give up the range
            .take_while(|(_end, begin)| begin.version >= self.start_version.unwrap_or(0))
            .peekable()
            .batching(|it| {
                match it.next() {
                    Some((end, mut begin)) => {
                        if end.version - begin.version >= self.target_job_size {
                            // cut big range short, this hopefully automatically skips load tests
                            let msg = if end.epoch - begin.epoch > 15 {
                                "!!! Need more snapshots !!!"
                            } else {
                                ""
                            };
                            Some((
                                true,
                                begin.version,
                                begin.version + self.target_job_size - 1,
                                format!(
                                    "Partial replay epoch {} - {}, {} txns starting from version {}, another {} versions omitted, until {}. {}",
                                    begin.epoch,
                                    end.epoch - 1,
                                    self.target_job_size,
                                    begin.version,
                                    end.version - begin.version - self.target_job_size,
                                    end.version,
                                    msg
                                )
                            ))
                        } else {
                            while let Some((_prev_end, prev_begin)) = it.peek() {
                                if end.version - prev_begin.version > self.target_job_size {
                                    break;
                                }
                                begin = prev_begin;
                                let _ = it.next();
                            }
                            Some((
                                false,
                                begin.version,
                                end.version - 1,
                                format!(
                                    "Replay epoch {} - {}, {} txns starting from version {}.",
                                    begin.epoch,
                                    end.epoch - 1,
                                    end.version - begin.version,
                                    begin.version,
                                )
                            ))
                        }
                    },
                    None => None,
                }
            }).collect_vec();

        // Deal with ranges_to_skip: to simplify things, we skip entire jobs instead of trimming them
        let mut ranges_to_skip = self
            .ranges_to_skip
            .iter()
            .flat_map(|range| {
                if range.is_empty() {
                    return None;
                }
                let (begin, end) = range
                    .split('-')
                    .map(|v| v.parse::<Version>().expect("Malformed range."))
                    .collect_tuple()
                    .expect("Malformed range.");
                assert!(begin <= end, "Malformed Range.");
                Some((begin, end))
            })
            .sorted()
            .rev()
            .peekable();

        let job_ranges = job_ranges
            .into_iter()
            .filter(|(_, first, last, _)| {
                while let Some((skip_first, skip_last)) = ranges_to_skip.peek() {
                    if *skip_first > *last {
                        let _ = ranges_to_skip.next();
                    } else {
                        return *skip_last < *first;
                    }
                }
                true
            })
            .collect_vec();

        info!(
            "Generated {} jobs. Now distribute them evenly between outputs.",
            job_ranges.len()
        );

        let mut outputs = vec![vec![]; self.output_json_files.len()];
        let mut job_idx = -1;
        zip(job_ranges, (0..self.output_json_files.len()).cycle()).for_each(
            |((partial, first, last, desc), output_idx)| {
                job_idx += 1;
                let suffix = if partial { "-partial" } else { "" };
                let job = format!("{output_idx}-{job_idx}{suffix} {first} {last} {desc}");
                outputs[output_idx].push(job);
            },
        );

        zip_eq(self.output_json_files.iter(), outputs.into_iter()).try_for_each(
            |(path, jobs)| {
                info!("Writing to {:?}", path);
                info!("{}", serde_json::to_string_pretty(&jobs)?);
                std::fs::File::create(path)?.write_all(&serde_json::to_vec(&jobs)?)
            },
        )?;

        Ok(())
    }
}
