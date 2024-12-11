// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    types::executed_chunk::ExecutedChunk,
    workflow::{do_get_execution_output::DoGetExecutionOutput, ApplyExecutionOutput},
};
use anyhow::{anyhow, ensure, format_err, Result};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_storage_interface::{
    state_store::state_view::{
        async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView,
    },
    DbReaderWriter, DbWriter, LedgerSummary,
};
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    aggregate_signature::AggregateSignature,
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    block_info::{BlockInfo, GENESIS_EPOCH, GENESIS_ROUND, GENESIS_TIMESTAMP_USECS},
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::ConfigurationResource,
    state_store::{state_key::StateKey, StateViewId, TStateView},
    timestamp::TimestampResource,
    transaction::Transaction,
    waypoint::Waypoint,
};
use aptos_vm::VMBlockExecutor;
use std::sync::Arc;

pub fn generate_waypoint<V: VMBlockExecutor>(
    db: &DbReaderWriter,
    genesis_txn: &Transaction,
) -> Result<Waypoint> {
    let ledger_summary = db.reader.get_pre_committed_ledger_summary()?;

    let committer = calculate_genesis::<V>(db, ledger_summary, genesis_txn)?;
    Ok(committer.waypoint)
}

/// If current version + 1 != waypoint.version(), return Ok(false) indicating skipping the txn.
/// otherwise apply the txn and commit it if the result matches the waypoint.
/// Returns Ok(true) if committed otherwise Err.
pub fn maybe_bootstrap<V: VMBlockExecutor>(
    db: &DbReaderWriter,
    genesis_txn: &Transaction,
    waypoint: Waypoint,
) -> Result<Option<LedgerInfoWithSignatures>> {
    let ledger_summary = db.reader.get_pre_committed_ledger_summary()?;
    // if the waypoint is not targeted with the genesis txn, it may be either already bootstrapped, or
    // aiming for state sync to catch up.
    if ledger_summary.version().map_or(0, |v| v + 1) != waypoint.version() {
        info!(waypoint = %waypoint, "Skip genesis txn.");
        return Ok(None);
    }

    let committer = calculate_genesis::<V>(db, ledger_summary, genesis_txn)?;
    ensure!(
        waypoint == committer.waypoint(),
        "Waypoint verification failed. Expected {:?}, got {:?}.",
        waypoint,
        committer.waypoint(),
    );
    let ledger_info = committer.output.ledger_info_opt.clone();
    committer.commit()?;
    Ok(ledger_info)
}

pub struct GenesisCommitter {
    db: Arc<dyn DbWriter>,
    output: ExecutedChunk,
    waypoint: Waypoint,
}

impl GenesisCommitter {
    pub fn new(db: Arc<dyn DbWriter>, output: ExecutedChunk) -> Result<Self> {
        let ledger_info = output
            .ledger_info_opt
            .as_ref()
            .ok_or_else(|| anyhow!("LedgerInfo missing."))?
            .ledger_info();
        let waypoint = Waypoint::new_epoch_boundary(ledger_info)?;

        Ok(Self {
            db,
            output,
            waypoint,
        })
    }

    pub fn waypoint(&self) -> Waypoint {
        self.waypoint
    }

    pub fn commit(self) -> Result<()> {
        self.db.save_transactions(
            self.output
                .output
                .expect_complete_result()
                .as_chunk_to_commit(),
            self.output.ledger_info_opt.as_ref(),
            true, /* sync_commit */
        )?;
        info!("Genesis commited.");
        // DB bootstrapped, avoid anything that could fail after this.

        Ok(())
    }
}

pub fn calculate_genesis<V: VMBlockExecutor>(
    db: &DbReaderWriter,
    ledger_summary: LedgerSummary,
    genesis_txn: &Transaction,
) -> Result<GenesisCommitter> {
    // DB bootstrapper works on either an empty transaction accumulator or an existing block chain.
    // In the very extreme and sad situation of losing quorum among validators, we refer to the
    // second use case said above.
    let genesis_version = ledger_summary.version().map_or(0, |v| v + 1);
    let base_state_view = ledger_summary.verified_state_view(
        StateViewId::Miscellaneous,
        Arc::clone(&db.reader),
        Arc::new(AsyncProofFetcher::new(db.reader.clone())),
    )?;

    let epoch = if genesis_version == 0 {
        GENESIS_EPOCH
    } else {
        get_state_epoch(&base_state_view)?
    };

    let execution_output = DoGetExecutionOutput::by_transaction_execution::<V>(
        &V::new(),
        vec![genesis_txn.clone().into()].into(),
        base_state_view,
        BlockExecutorConfigFromOnchain::new_no_block_limit(),
        TransactionSliceMetadata::unknown(),
    )?;
    ensure!(
        execution_output.num_transactions_to_commit() != 0,
        "Genesis txn execution failed."
    );
    ensure!(
        execution_output.next_epoch_state.is_some(),
        "Genesis txn didn't output reconfig event."
    );

    let output = ApplyExecutionOutput::run(execution_output, &ledger_summary)?;
    let timestamp_usecs = if genesis_version == 0 {
        // TODO(aldenhu): fix existing tests before using real timestamp and check on-chain epoch.
        GENESIS_TIMESTAMP_USECS
    } else {
        let state_view = CachedStateView::new(
            StateViewId::Miscellaneous,
            Arc::clone(&db.reader),
            output.execution_output.next_version(),
            output.expect_result_state().current.clone(),
            Arc::new(AsyncProofFetcher::new(db.reader.clone())),
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

    let ledger_info_with_sigs = LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(
                epoch,
                GENESIS_ROUND,
                genesis_block_id(),
                output
                    .expect_ledger_update_output()
                    .transaction_accumulator
                    .root_hash(),
                genesis_version,
                timestamp_usecs,
                output.execution_output.next_epoch_state.clone(),
            ),
            genesis_block_id(), /* consensus_data_hash */
        ),
        AggregateSignature::empty(), /* signatures */
    );
    let executed_chunk = ExecutedChunk {
        output,
        ledger_info_opt: Some(ledger_info_with_sigs),
    };

    let committer = GenesisCommitter::new(db.writer.clone(), executed_chunk)?;
    info!(
        "Genesis calculated: ledger_info_with_sigs {:?}, waypoint {:?}",
        &committer.output.ledger_info_opt, committer.waypoint,
    );
    Ok(committer)
}

fn get_state_timestamp(state_view: &CachedStateView) -> Result<u64> {
    let rsrc_bytes = &state_view
        .get_state_value_bytes(&StateKey::resource_typed::<TimestampResource>(
            &CORE_CODE_ADDRESS,
        )?)?
        .ok_or_else(|| format_err!("TimestampResource missing."))?;
    let rsrc = bcs::from_bytes::<TimestampResource>(rsrc_bytes)?;
    Ok(rsrc.timestamp.microseconds)
}

fn get_state_epoch(state_view: &CachedStateView) -> Result<u64> {
    let rsrc_bytes = &state_view
        .get_state_value_bytes(&StateKey::on_chain_config::<ConfigurationResource>()?)?
        .ok_or_else(|| format_err!("ConfigurationResource missing."))?;
    let rsrc = bcs::from_bytes::<ConfigurationResource>(rsrc_bytes)?;
    Ok(rsrc.epoch())
}

fn genesis_block_id() -> HashValue {
    HashValue::zero()
}
