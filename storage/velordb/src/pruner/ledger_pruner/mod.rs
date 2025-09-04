// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod event_store_pruner;
mod ledger_metadata_pruner;
pub(crate) mod ledger_pruner_manager;
mod persisted_auxiliary_info_pruner;
mod transaction_accumulator_pruner;
mod transaction_auxiliary_data_pruner;
mod transaction_info_pruner;
mod transaction_pruner;
mod write_set_pruner;

use crate::{
    ledger_db::LedgerDb,
    metrics::PRUNER_VERSIONS,
    pruner::{
        db_pruner::DBPruner,
        db_sub_pruner::DBSubPruner,
        ledger_pruner::{
            event_store_pruner::EventStorePruner, ledger_metadata_pruner::LedgerMetadataPruner,
            persisted_auxiliary_info_pruner::PersistedAuxiliaryInfoPruner,
            transaction_accumulator_pruner::TransactionAccumulatorPruner,
            transaction_auxiliary_data_pruner::TransactionAuxiliaryDataPruner,
            transaction_info_pruner::TransactionInfoPruner, transaction_pruner::TransactionPruner,
            write_set_pruner::WriteSetPruner,
        },
    },
    transaction_store::TransactionStore,
};
use anyhow::anyhow;
use velor_db_indexer::db_indexer::InternalIndexerDB;
use velor_experimental_runtimes::thread_manager::THREAD_MANAGER;
use velor_logger::info;
use velor_storage_interface::Result;
use velor_types::transaction::{AtomicVersion, Version};
use rayon::prelude::*;
use std::{
    cmp::min,
    sync::{atomic::Ordering, Arc},
};

pub const LEDGER_PRUNER_NAME: &str = "ledger_pruner";

/// Responsible for pruning everything except for the state tree.
pub(crate) struct LedgerPruner {
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,

    progress: AtomicVersion,

    ledger_metadata_pruner: Box<LedgerMetadataPruner>,

    sub_pruners: Vec<Box<dyn DBSubPruner + Send + Sync>>,
}

impl DBPruner for LedgerPruner {
    fn name(&self) -> &'static str {
        LEDGER_PRUNER_NAME
    }

    fn prune(&self, max_versions: usize) -> Result<Version> {
        let mut progress = self.progress();
        let target_version = self.target_version();

        while progress < target_version {
            let current_batch_target_version =
                min(progress + max_versions as Version, target_version);

            info!(
                progress = progress,
                target_version = current_batch_target_version,
                "Pruning ledger data."
            );
            self.ledger_metadata_pruner
                .prune(progress, current_batch_target_version)?;

            THREAD_MANAGER.get_background_pool().install(|| {
                self.sub_pruners.par_iter().try_for_each(|sub_pruner| {
                    sub_pruner
                        .prune(progress, current_batch_target_version)
                        .map_err(|err| anyhow!("{} failed to prune: {err}", sub_pruner.name()))
                })
            })?;

            progress = current_batch_target_version;
            self.record_progress(progress);
            info!(progress = progress, "Pruning ledger data is done.");
        }

        Ok(target_version)
    }

    fn progress(&self) -> Version {
        self.progress.load(Ordering::SeqCst)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&["ledger_pruner", "target"])
            .set(target_version as i64);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::SeqCst)
    }

    fn record_progress(&self, progress: Version) {
        self.progress.store(progress, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&["ledger_pruner", "progress"])
            .set(progress as i64);
    }
}

impl LedgerPruner {
    pub fn new(
        ledger_db: Arc<LedgerDb>,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        info!(name = LEDGER_PRUNER_NAME, "Initializing...");

        let ledger_metadata_pruner = Box::new(
            LedgerMetadataPruner::new(ledger_db.metadata_db_arc())
                .expect("Failed to initialize ledger_metadata_pruner."),
        );

        let metadata_progress = ledger_metadata_pruner.progress()?;

        info!(
            metadata_progress = metadata_progress,
            "Created ledger metadata pruner, start catching up all sub pruners."
        );

        let transaction_store = Arc::new(TransactionStore::new(Arc::clone(&ledger_db)));

        let event_store_pruner = Box::new(EventStorePruner::new(
            Arc::clone(&ledger_db),
            metadata_progress,
            internal_indexer_db.clone(),
        )?);
        let persisted_auxiliary_info_pruner = Box::new(PersistedAuxiliaryInfoPruner::new(
            Arc::clone(&ledger_db),
            metadata_progress,
        )?);
        let transaction_accumulator_pruner = Box::new(TransactionAccumulatorPruner::new(
            Arc::clone(&ledger_db),
            metadata_progress,
        )?);

        let transaction_auxiliary_data_pruner = Box::new(TransactionAuxiliaryDataPruner::new(
            Arc::clone(&ledger_db),
            metadata_progress,
        )?);

        let transaction_info_pruner = Box::new(TransactionInfoPruner::new(
            Arc::clone(&ledger_db),
            metadata_progress,
        )?);
        let transaction_pruner = Box::new(TransactionPruner::new(
            Arc::clone(&transaction_store),
            Arc::clone(&ledger_db),
            metadata_progress,
            internal_indexer_db,
        )?);
        let write_set_pruner = Box::new(WriteSetPruner::new(
            Arc::clone(&ledger_db),
            metadata_progress,
        )?);

        let pruner = LedgerPruner {
            target_version: AtomicVersion::new(metadata_progress),
            progress: AtomicVersion::new(metadata_progress),
            ledger_metadata_pruner,
            sub_pruners: vec![
                event_store_pruner,
                persisted_auxiliary_info_pruner,
                transaction_accumulator_pruner,
                transaction_auxiliary_data_pruner,
                transaction_info_pruner,
                transaction_pruner,
                write_set_pruner,
            ],
        };

        info!(
            name = pruner.name(),
            progress = metadata_progress,
            "Initialized."
        );

        Ok(pruner)
    }
}
