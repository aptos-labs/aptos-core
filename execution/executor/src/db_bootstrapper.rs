// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::components::chunk_output::ChunkOutput;
use anyhow::{anyhow, ensure, format_err, Result};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_state_view::{StateView, StateViewId};
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::{
    access_path::AccessPath,
    account_config::CORE_CODE_ADDRESS,
    block_info::{BlockInfo, GENESIS_EPOCH, GENESIS_ROUND, GENESIS_TIMESTAMP_USECS},
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::ConfigurationResource,
    state_store::state_key::StateKey,
    timestamp::TimestampResource,
    transaction::{Transaction, Version},
    waypoint::Waypoint,
};
use aptos_vm::VMExecutor;
use executor_types::ExecutedChunk;
use move_deps::move_core_types::move_resource::MoveResource;
use std::sync::Arc;
use storage_interface::{
    cached_state_view::CachedStateView, sync_proof_fetcher::SyncProofFetcher, DbReaderWriter,
    DbWriter, ExecutedTrees,
};

pub fn generate_waypoint<V: VMExecutor>(
    db: &DbReaderWriter,
    genesis_txn: &Transaction,
) -> Result<Waypoint> {
    let executed_trees = db.reader.get_latest_executed_trees()?;

    let committer = calculate_genesis::<V>(db, executed_trees, genesis_txn)?;
    Ok(committer.waypoint)
}

/// If current version + 1 != waypoint.version(), return Ok(false) indicating skipping the txn.
/// otherwise apply the txn and commit it if the result matches the waypoint.
/// Returns Ok(true) if committed otherwise Err.
pub fn maybe_bootstrap<V: VMExecutor>(
    db: &DbReaderWriter,
    genesis_txn: &Transaction,
    waypoint: Waypoint,
) -> Result<bool> {
    let executed_trees = db.reader.get_latest_executed_trees()?;
    // if the waypoint is not targeted with the genesis txn, it may be either already bootstrapped, or
    // aiming for state sync to catch up.
    if executed_trees.version().map_or(0, |v| v + 1) != waypoint.version() {
        info!(waypoint = %waypoint, "Skip genesis txn.");
        return Ok(false);
    }

    let committer = calculate_genesis::<V>(db, executed_trees, genesis_txn)?;
    ensure!(
        waypoint == committer.waypoint(),
        "Waypoint verification failed. Expected {:?}, got {:?}.",
        waypoint,
        committer.waypoint(),
    );
    committer.commit()?;
    Ok(true)
}

pub struct GenesisCommitter {
    db: Arc<dyn DbWriter>,
    output: ExecutedChunk,
    base_state_version: Option<Version>,
    waypoint: Waypoint,
}

impl GenesisCommitter {
    pub fn new(
        db: Arc<dyn DbWriter>,
        output: ExecutedChunk,
        base_state_version: Option<Version>,
    ) -> Result<Self> {
        let ledger_info = output
            .ledger_info
            .as_ref()
            .ok_or_else(|| anyhow!("LedgerInfo missing."))?
            .ledger_info();
        let waypoint = Waypoint::new_epoch_boundary(ledger_info)?;

        Ok(Self {
            db,
            output,
            waypoint,
            base_state_version,
        })
    }

    pub fn waypoint(&self) -> Waypoint {
        self.waypoint
    }

    pub fn commit(self) -> Result<()> {
        self.db.save_transactions(
            &self.output.transactions_to_commit()?,
            self.output.result_view.txn_accumulator().version(),
            self.base_state_version,
            self.output.ledger_info.as_ref(),
            true, /* sync_commit */
            self.output.result_view.state().clone(),
        )?;
        info!("Genesis commited.");
        // DB bootstrapped, avoid anything that could fail after this.

        Ok(())
    }
}

pub fn calculate_genesis<V: VMExecutor>(
    db: &DbReaderWriter,
    executed_trees: ExecutedTrees,
    genesis_txn: &Transaction,
) -> Result<GenesisCommitter> {
    // DB bootstrapper works on either an empty transaction accumulator or an existing block chain.
    // In the very extreme and sad situation of losing quorum among validators, we refer to the
    // second use case said above.
    let genesis_version = executed_trees.version().map_or(0, |v| v + 1);
    let base_state_view = executed_trees.verified_state_view(
        StateViewId::Miscellaneous,
        Arc::clone(&db.reader),
        Arc::new(SyncProofFetcher::new(db.reader.clone())),
    )?;

    let epoch = if genesis_version == 0 {
        GENESIS_EPOCH
    } else {
        get_state_epoch(&base_state_view)?
    };

    let (mut output, _, _) =
        ChunkOutput::by_transaction_execution::<V>(vec![genesis_txn.clone()], base_state_view)?
            .apply_to_ledger(&executed_trees)?;
    ensure!(
        !output.to_commit.is_empty(),
        "Genesis txn execution failed."
    );

    let timestamp_usecs = if genesis_version == 0 {
        // TODO(aldenhu): fix existing tests before using real timestamp and check on-chain epoch.
        GENESIS_TIMESTAMP_USECS
    } else {
        let state_view = output.result_view.verified_state_view(
            StateViewId::Miscellaneous,
            Arc::clone(&db.reader),
            Arc::new(SyncProofFetcher::new(db.reader.clone())),
        )?;
        let next_epoch = epoch
            .checked_add(1)
            .ok_or_else(|| format_err!("integer overflow occurred"))?;
        ensure!(
            next_epoch == get_state_epoch(&state_view)?,
            "Genesis txn didn't bump epoch."
        );
        get_state_timestamp(&state_view)?
    };
    ensure!(
        output.next_epoch_state.is_some(),
        "Genesis txn didn't output reconfig event."
    );

    let ledger_info_with_sigs = LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(
                epoch,
                GENESIS_ROUND,
                genesis_block_id(),
                output.result_view.txn_accumulator().root_hash(),
                genesis_version,
                timestamp_usecs,
                output.next_epoch_state.clone(),
            ),
            genesis_block_id(), /* consensus_data_hash */
        ),
        AggregateSignature::empty(), /* signatures */
    );
    output.ledger_info = Some(ledger_info_with_sigs);

    let committer = GenesisCommitter::new(
        db.writer.clone(),
        output,
        executed_trees.state().base_version,
    )?;
    info!(
        "Genesis calculated: ledger_info_with_sigs {:?}, waypoint {:?}",
        &committer.output.ledger_info, committer.waypoint,
    );
    Ok(committer)
}

fn get_state_timestamp(state_view: &CachedStateView) -> Result<u64> {
    let rsrc_bytes = &state_view
        .get_state_value(&StateKey::AccessPath(AccessPath::new(
            CORE_CODE_ADDRESS,
            TimestampResource::resource_path(),
        )))?
        .ok_or_else(|| format_err!("TimestampResource missing."))?;
    let rsrc = bcs::from_bytes::<TimestampResource>(rsrc_bytes)?;
    Ok(rsrc.timestamp.microseconds)
}

fn get_state_epoch(state_view: &CachedStateView) -> Result<u64> {
    let rsrc_bytes = &state_view
        .get_state_value(&StateKey::AccessPath(AccessPath::new(
            CORE_CODE_ADDRESS,
            ConfigurationResource::resource_path(),
        )))?
        .ok_or_else(|| format_err!("ConfigurationResource missing."))?;
    let rsrc = bcs::from_bytes::<ConfigurationResource>(rsrc_bytes)?;
    Ok(rsrc.epoch())
}

fn genesis_block_id() -> HashValue {
    HashValue::zero()
}
