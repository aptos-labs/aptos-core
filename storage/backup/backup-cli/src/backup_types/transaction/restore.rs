// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::{
        epoch_ending::restore::EpochHistory,
        transaction::manifest::{TransactionBackup, TransactionChunk},
    },
    metrics::{
        restore::{TRANSACTION_REPLAY_VERSION, TRANSACTION_SAVE_VERSION},
        verify::VERIFY_TRANSACTION_VERSION,
        OTHER_TIMERS_SECONDS,
    },
    storage::{BackupStorage, FileHandle},
    utils::{
        error_notes::ErrorNotes,
        read_record_bytes::ReadRecordBytes,
        storage_ext::BackupStorageExt,
        stream::{StreamX, TryStreamX},
        GlobalRestoreOptions, RestoreRunMode,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_logger::prelude::*;
use aptos_types::{
    account_config::{DepositEvent, CORE_CODE_ADDRESS},
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{TransactionAccumulatorRangeProof, TransactionInfoListWithProof},
    stake_pool::DistributeRewardsEvent,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{
        ExecutionStatus, Transaction, TransactionInfo, TransactionListWithProof,
        TransactionPayload, Version,
    },
    write_set::{WriteOp, WriteSet},
};
use aptos_vm::AptosVM;
use aptosdb::backup::restore_handler::RestoreHandler;
use clap::Parser;
use executor::chunk_executor::ChunkExecutor;
use executor_types::TransactionReplayer;
use futures::{
    future,
    future::TryFutureExt,
    stream,
    stream::{Peekable, Stream, TryStreamExt},
    StreamExt,
};
use itertools::zip_eq;
use move_core_types::{account_address::AccountAddress, transaction_argument::TransactionArgument};
use std::{
    cmp::{max, min},
    io::Write,
    pin::Pin,
    sync::Arc,
    time::Instant,
};
use storage_interface::DbReaderWriter;
use tokio::io::BufReader;

const BATCH_SIZE: usize = if cfg!(test) { 2 } else { 10000 };
static MINTER_SCRIPT: &[u8] = include_bytes!("minter.mv");

#[derive(Parser)]
pub struct TransactionRestoreOpt {
    #[clap(long = "transaction-manifest")]
    pub manifest_handle: FileHandle,
    #[clap(
        long = "replay-transactions-from-version",
        help = "Transactions with this version and above will be replayed so state and events are \
        gonna pop up. Requires state at the version right before this to exist, either by \
        recovering a state snapshot, or previous transaction replay."
    )]
    pub replay_from_version: Option<Version>,
}

impl TransactionRestoreOpt {
    pub fn replay_from_version(&self) -> Version {
        self.replay_from_version.unwrap_or(Version::max_value())
    }
}

pub struct TransactionRestoreController {
    inner: TransactionRestoreBatchController,
}

#[allow(dead_code)]
struct LoadedChunk {
    pub manifest: TransactionChunk,
    pub txns: Vec<Transaction>,
    pub txn_infos: Vec<TransactionInfo>,
    pub event_vecs: Vec<Vec<ContractEvent>>,
    pub range_proof: TransactionAccumulatorRangeProof,
    pub ledger_info: LedgerInfoWithSignatures,
    pub write_sets: Vec<WriteSet>,
}

impl LoadedChunk {
    async fn load(
        manifest: TransactionChunk,
        storage: &Arc<dyn BackupStorage>,
        epoch_history: Option<&Arc<EpochHistory>>,
    ) -> Result<Self> {
        let mut file = BufReader::new(storage.open_for_read(&manifest.transactions).await?);
        let mut txns = Vec::new();
        let mut txn_infos = Vec::new();
        let mut event_vecs = Vec::new();
        let mut write_sets = Vec::new();

        while let Some(record_bytes) = file.read_record_bytes().await? {
            let (txn, txn_info, events, write_set): (_, _, _, WriteSet) =
                bcs::from_bytes(&record_bytes)?;
            txns.push(txn);
            txn_infos.push(txn_info);
            event_vecs.push(events);
            write_sets.push(write_set);
        }

        ensure!(
            manifest.first_version + (txns.len() as Version) == manifest.last_version + 1,
            "Number of items in chunks doesn't match that in manifest. first_version: {}, last_version: {}, items in chunk: {}",
            manifest.first_version,
            manifest.last_version,
            txns.len(),
        );

        let (range_proof, ledger_info) = storage
            .load_bcs_file::<(TransactionAccumulatorRangeProof, LedgerInfoWithSignatures)>(
                &manifest.proof,
            )
            .await?;
        if let Some(epoch_history) = epoch_history {
            epoch_history.verify_ledger_info(&ledger_info)?;
        }

        // make a `TransactionListWithProof` to reuse its verification code.
        let txn_list_with_proof = TransactionListWithProof::new(
            txns,
            Some(event_vecs),
            Some(manifest.first_version),
            TransactionInfoListWithProof::new(range_proof, txn_infos),
        );
        txn_list_with_proof.verify(ledger_info.ledger_info(), Some(manifest.first_version))?;
        // and disassemble it to get things back.
        let txns = txn_list_with_proof.transactions;
        let range_proof = txn_list_with_proof
            .proof
            .ledger_info_to_transaction_infos_proof;
        let txn_infos = txn_list_with_proof.proof.transaction_infos;
        let event_vecs = txn_list_with_proof.events.expect("unknown to be Some.");

        Ok(Self {
            manifest,
            txns,
            txn_infos,
            event_vecs,
            range_proof,
            ledger_info,
            write_sets,
        })
    }
}

impl TransactionRestoreController {
    pub fn new(
        opt: TransactionRestoreOpt,
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
        epoch_history: Option<Arc<EpochHistory>>,
    ) -> Self {
        let inner = TransactionRestoreBatchController::new(
            global_opt,
            storage,
            vec![opt.manifest_handle],
            opt.replay_from_version,
            epoch_history,
        );

        Self { inner }
    }

    pub async fn run(self) -> Result<()> {
        self.inner.run().await
    }
}

impl TransactionRestoreController {}

/// Takes a series of transaction backup manifests, preheat in parallel, then execute in order.
pub struct TransactionRestoreBatchController {
    global_opt: GlobalRestoreOptions,
    storage: Arc<dyn BackupStorage>,
    manifest_handles: Vec<FileHandle>,
    replay_from_version: Option<Version>,
    epoch_history: Option<Arc<EpochHistory>>,
}

impl TransactionRestoreBatchController {
    pub fn new(
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
        manifest_handles: Vec<FileHandle>,
        replay_from_version: Option<Version>,
        epoch_history: Option<Arc<EpochHistory>>,
    ) -> Self {
        Self {
            global_opt,
            storage,
            manifest_handles,
            replay_from_version,
            epoch_history,
        }
    }

    pub async fn run(self) -> Result<()> {
        let name = self.name();
        info!("{} started.", name);
        self.run_impl()
            .await
            .map_err(|e| anyhow!("{} failed: {}", name, e))?;
        info!("{} succeeded.", name);
        Ok(())
    }

    fn name(&self) -> String {
        format!("transaction {}", self.global_opt.run_mode.name())
    }

    async fn run_impl(self) -> Result<()> {
        if self.manifest_handles.is_empty() {
            return Ok(());
        }

        let loaded_chunk_stream = self.loaded_chunk_stream();
        // let first_version = self
        //     .confirm_or_save_frozen_subtrees(&mut loaded_chunk_stream)
        //     .await?;

        Self::verify_total_supply(loaded_chunk_stream).await?;
        // if let RestoreRunMode::Restore { restore_handler } = self.global_opt.run_mode.as_ref() {
        //     AptosVM::set_concurrency_level_once(self.global_opt.replay_concurrency_level);
        //     let txns_to_execute_stream = self
        //         .save_before_replay_version(first_version, loaded_chunk_stream, restore_handler)
        //         .await?;

        //     if let Some(txns_to_execute_stream) = txns_to_execute_stream {
        //         self.replay_transactions(restore_handler, txns_to_execute_stream)
        //             .await?;
        //     }
        // } else {
        //     Self::go_through_verified_chunks(loaded_chunk_stream, first_version).await?;
        // }
        Ok(())
    }

    async fn verify_total_supply(
        loaded_chunk_stream: impl Stream<Item = Result<LoadedChunk>>,
    ) -> anyhow::Result<()> {
        // Need this for last batch != 1000000
        // let mut a: usize = 0;

        let supply_handle: TableHandle = TableHandle(
            AccountAddress::from_hex_literal(
                "0x1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca",
            )
            .unwrap(),
        );
        let supply_key: Vec<u8> = vec![
            6, 25, 220, 41, 160, 170, 200, 250, 20, 103, 20, 5, 142, 141, 214, 210, 208, 243, 189,
            245, 246, 51, 25, 7, 191, 145, 243, 172, 216, 30, 105, 53,
        ];
        let state_key = StateKey::TableItem {
            handle: supply_handle,
            key: supply_key,
        };
        let another_minter_script = hex::decode("a11ceb0b050000000601000603060f05150b07203908592006790a00000001000202030301000104000200000500020003060c050301050001060c0d6170746f735f6163636f756e740a6170746f735f636f696e067369676e65720a616464726573735f6f66046d696e74087472616e7366657200000000000000000000000000000000000000000000000000000000000000010308a0860100000000000000010e0a0011000c030a000b030a0207001611010b000b010b02110202").unwrap();
        let mut prev_total_supply: u128 = 0;

        loaded_chunk_stream
            .for_each(|res| {
                let chunk = res.unwrap();
                let num_txns = chunk.txns.len();

                let (total_burnt, total_minted) = chunk
                    .txns
                    .iter()
                    .enumerate()
                    .map(|(index, txn)| {
                        let txn_info = &chunk.txn_infos[index];
                        let events = &chunk.event_vecs[index];
                        let write_set = &chunk.write_sets[index];
                        // if a + index > 281199 {
                        //     return (0, 0);
                        // }

                        // First, find wrapped mints - the only place they can come from is
                        // reward redistribution.
                        let mut minted: u64 = 0;
                        let mut num_deposit_events = 0;
                        for event in events {
                            if let Ok(rewards_event) = DistributeRewardsEvent::try_from(event) {
                                minted += rewards_event.rewards_amount;
                                continue;
                            }
                            if let Ok(_) = DepositEvent::try_from(event) {
                                num_deposit_events += 1;
                            }
                        }

                        // Now calculate how much fees was burnt and also check if the payload
                        // is a minting script.
                        let (burn, mint) = match txn {
                            Transaction::UserTransaction(signed_txn) => {
                                let burnt = signed_txn.gas_unit_price() * txn_info.gas_used();

                                if *txn_info.status() == ExecutionStatus::Success {
                                    // Additionally, we might get some mints from entry functions.
                                    if let TransactionPayload::EntryFunction(func) =
                                        signed_txn.payload()
                                    {
                                        let core_module =
                                            func.module().address().eq(&CORE_CODE_ADDRESS);
                                        let coin_module =
                                            func.module().name().as_str().eq("aptos_coin");
                                        let mint_func = func.function().as_str().eq("mint");
                                        if core_module && coin_module && mint_func {
                                            let amount: u64 = bcs::from_bytes(&func.args()[1])
                                                .expect(
                                                "unable to deserialize the amount of minted coins",
                                            );
                                            minted += amount
                                        }
                                    }

                                    if let TransactionPayload::Script(script) = signed_txn.payload()
                                    {
                                        if (script.code() == MINTER_SCRIPT
                                            || script.code() == &another_minter_script)
                                            && num_deposit_events == 2
                                        {
                                            match &script.args()[1] {
                                                TransactionArgument::U64(amount) => {
                                                    minted += amount + 100000; // GAS BUFFER
                                                }
                                                _ => unreachable!(),
                                            }
                                        }
                                    }
                                }
                                (burnt as u128, minted as u128)
                            }
                            Transaction::GenesisTransaction(_) => (0, 18448344073709551615),
                            _ => (0, minted as u128),
                        };
                        if burn == 0 && mint == 0 {
                            return (burn, mint);
                        }
                        let current_total_supply: u128 =
                            match write_set.clone().into_mut().get(&state_key).unwrap() {
                                WriteOp::Creation(value) => bcs::from_bytes(value).unwrap(),
                                WriteOp::Modification(value) => bcs::from_bytes(value).unwrap(),
                                _ => unreachable!(),
                            };
                        let valid =
                            prev_total_supply + mint as u128 - burn as u128 == current_total_supply;
                        if !valid {
                            println!("Version: {}", chunk.manifest.first_version + index as u64);
                            println!("Transaction: {:?}", txn);
                            println!("burn {} mint {}", burn, mint);
                            println!(
                                "Expected {}, aggregator {}",
                                prev_total_supply + mint as u128 - burn as u128,
                                current_total_supply
                            );
                            panic!();
                        }
                        prev_total_supply = current_total_supply;
                        (burn, mint)
                    })
                    .fold((0_u128, 0_u128), |(acc_b, acc_m), (b, m)| {
                        (acc_b + b as u128, acc_m + m as u128)
                    });

                // a += num_txns;

                println!(
                    "total burnt {} mint {} for chunk {}-{}",
                    total_burnt,
                    total_minted,
                    chunk.manifest.first_version,
                    chunk.manifest.last_version
                );
                future::ready(())
            })
            .await;
        Ok(())
    }

    fn loaded_chunk_stream(&self) -> Peekable<impl Stream<Item = Result<LoadedChunk>>> {
        let con = self.global_opt.concurrent_downloads;

        let manifest_handle_stream = stream::iter(self.manifest_handles.clone().into_iter());

        let storage = self.storage.clone();
        let manifest_stream = manifest_handle_stream
            .map(move |hdl| {
                let storage = storage.clone();
                async move { storage.load_json_file(&hdl).await.err_notes(&hdl) }
            })
            .buffered_x(con * 3, con)
            .and_then(|m: TransactionBackup| future::ready(m.verify().map(|_| m)));

        let target_version = self.global_opt.target_version;
        let chunk_manifest_stream = manifest_stream
            .map_ok(|m| stream::iter(m.chunks.into_iter().map(Result::<_>::Ok)))
            .try_flatten()
            .try_take_while(move |c| future::ready(Ok(c.first_version <= target_version)))
            .scan(0, |last_chunk_last_version, chunk_res| {
                let res = match &chunk_res {
                    Ok(chunk) => {
                        if *last_chunk_last_version != 0
                            && chunk.first_version != *last_chunk_last_version + 1
                        {
                            Some(Err(anyhow!(
                                "Chunk range not consecutive. expecting {}, got {}",
                                *last_chunk_last_version + 1,
                                chunk.first_version
                            )))
                        } else {
                            *last_chunk_last_version = chunk.last_version;
                            Some(chunk_res)
                        }
                    }
                    Err(_) => Some(chunk_res),
                };
                future::ready(res)
            });

        let storage = self.storage.clone();
        let epoch_history = self.epoch_history.clone();
        chunk_manifest_stream
            .and_then(move |chunk| {
                let storage = storage.clone();
                let epoch_history = epoch_history.clone();
                future::ok(async move {
                    tokio::task::spawn(async move {
                        LoadedChunk::load(chunk, &storage, epoch_history.as_ref()).await
                    })
                    .err_into::<anyhow::Error>()
                    .await
                })
            })
            .try_buffered_x(con * 2, con)
            .and_then(future::ready)
            .peekable()
    }

    async fn confirm_or_save_frozen_subtrees(
        &self,
        loaded_chunk_stream: &mut Peekable<impl Unpin + Stream<Item = Result<LoadedChunk>>>,
    ) -> Result<Version> {
        let first_chunk = Pin::new(loaded_chunk_stream)
            .peek()
            .await
            .ok_or_else(|| anyhow!("LoadedChunk stream is empty."))?
            .as_ref()
            .map_err(|e| anyhow!("Error: {}", e))?;

        if let RestoreRunMode::Restore { restore_handler } = self.global_opt.run_mode.as_ref() {
            restore_handler.confirm_or_save_frozen_subtrees(
                first_chunk.manifest.first_version,
                first_chunk.range_proof.left_siblings(),
            )?;
        }

        Ok(first_chunk.manifest.first_version)
    }

    async fn save_before_replay_version(
        &self,
        global_first_version: Version,
        loaded_chunk_stream: impl Stream<Item = Result<LoadedChunk>> + Unpin,
        restore_handler: &RestoreHandler,
    ) -> Result<Option<impl Stream<Item = Result<(Transaction, TransactionInfo)>>>> {
        let next_expected_version = self
            .global_opt
            .run_mode
            .get_next_expected_transaction_version()?;
        let start = Instant::now();

        let restore_handler_clone = restore_handler.clone();
        // DB doesn't allow replaying anything before what's in DB already.
        //
        // TODO: notice that ideals we detect and avoid calling rh.save_transactions() for txns
        //       before `first_to_replay` calculated below, but we don't deal with it for now,
        //       because unlike replaying, that's allowed by the DB. Need to follow up later.
        let first_to_replay = max(
            self.replay_from_version.unwrap_or(Version::MAX),
            next_expected_version,
        );
        let target_version = self.global_opt.target_version;

        let mut txns_to_execute_stream = loaded_chunk_stream
            .and_then(move |chunk| {
                let restore_handler = restore_handler_clone.clone();
                future::ok(async move {
                    let LoadedChunk {
                        manifest:
                            TransactionChunk {
                                first_version,
                                mut last_version,
                                transactions: _,
                                proof: _,
                            },
                        mut txns,
                        mut txn_infos,
                        mut event_vecs,
                        range_proof: _,
                        ledger_info: _,
                        write_sets: _,
                    } = chunk;

                    if target_version < last_version {
                        let num_to_keep = (target_version - first_version + 1) as usize;
                        txns.drain(num_to_keep..);
                        txn_infos.drain(num_to_keep..);
                        event_vecs.drain(num_to_keep..);
                        last_version = target_version;
                    }

                    if first_version < first_to_replay {
                        let num_to_save =
                            (min(first_to_replay, last_version + 1) - first_version) as usize;
                        let txns_to_save: Vec<_> = txns.drain(..num_to_save).collect();
                        let txn_infos_to_save: Vec<_> = txn_infos.drain(..num_to_save).collect();
                        let event_vecs_to_save: Vec<_> = event_vecs.drain(..num_to_save).collect();

                        tokio::task::spawn_blocking(move || {
                            restore_handler.save_transactions(
                                first_version,
                                &txns_to_save,
                                &txn_infos_to_save,
                                &event_vecs_to_save,
                            )
                        })
                        .await??;
                        let last_saved = first_version + num_to_save as u64 - 1;
                        TRANSACTION_SAVE_VERSION.set(last_saved as i64);
                        info!(
                            version = last_saved,
                            accumulative_tps = (last_saved - global_first_version + 1) as f64
                                / start.elapsed().as_secs_f64(),
                            "Transactions saved."
                        );
                    }

                    Ok(stream::iter(
                        zip_eq(txns, txn_infos).into_iter().map(Result::<_>::Ok),
                    ))
                })
            })
            .try_buffered_x(self.global_opt.concurrent_downloads, 1)
            .try_flatten()
            .peekable();

        // Finish saving transactions that are not to be replayed.
        let first_txn_to_replay = {
            Pin::new(&mut txns_to_execute_stream)
                .peek()
                .await
                .map(|res| res.as_ref().map_err(|e| anyhow!("Error: {}", e)))
                .transpose()?
                .map(|_| ())
        };

        Ok(first_txn_to_replay.map(|_| txns_to_execute_stream))
    }

    async fn replay_transactions(
        &self,
        restore_handler: &RestoreHandler,
        txns_to_execute_stream: impl Stream<Item = Result<(Transaction, TransactionInfo)>>,
    ) -> Result<()> {
        let first_version = self.replay_from_version.unwrap();
        restore_handler.reset_state_store();
        let replay_start = Instant::now();
        let db = DbReaderWriter::from_arc(Arc::clone(&restore_handler.aptosdb));
        let chunk_replayer = Arc::new(ChunkExecutor::<AptosVM>::new(db));

        let db_commit_stream = txns_to_execute_stream
            .try_chunks(BATCH_SIZE)
            .err_into::<anyhow::Error>()
            .map_ok(|chunk| {
                let (txns, txn_infos): (Vec<_>, Vec<_>) = chunk.into_iter().unzip();
                let chunk_replayer = chunk_replayer.clone();
                async move {
                    let _timer = OTHER_TIMERS_SECONDS
                        .with_label_values(&["replay_txn_chunk"])
                        .start_timer();
                    tokio::task::spawn_blocking(move || chunk_replayer.replay(txns, txn_infos))
                        .err_into::<anyhow::Error>()
                        .await
                }
            })
            .try_buffered_x(self.global_opt.concurrent_downloads, 1)
            .and_then(future::ready);

        db_commit_stream
            .and_then(|()| {
                let chunk_replayer = chunk_replayer.clone();
                async move {
                    let _timer = OTHER_TIMERS_SECONDS
                        .with_label_values(&["commit_txn_chunk"])
                        .start_timer();
                    tokio::task::spawn_blocking(move || {
                        let committed_chunk = chunk_replayer.commit()?;
                        let v = committed_chunk.result_view.version().unwrap_or(0);
                        TRANSACTION_REPLAY_VERSION.set(v as i64);
                        info!(
                            version = v,
                            accumulative_tps = (v - first_version + 1) as f64
                                / replay_start.elapsed().as_secs_f64(),
                            "Transactions replayed."
                        );
                        Ok(())
                    })
                    .await?
                }
            })
            .try_fold((), |(), ()| future::ok(()))
            .await
    }

    async fn go_through_verified_chunks(
        loaded_chunk_stream: impl Stream<Item = Result<LoadedChunk>>,
        first_version: Version,
    ) -> Result<()> {
        let start = Instant::now();
        loaded_chunk_stream
            .try_fold((), |(), chunk| {
                let v = chunk.manifest.last_version;
                VERIFY_TRANSACTION_VERSION.set(v as i64);
                info!(
                    version = v,
                    accumulative_tps =
                        (v - first_version + 1) as f64 / start.elapsed().as_secs_f64(),
                    "Transactions verified."
                );
                future::ok(())
            })
            .await
    }
}
