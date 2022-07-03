// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    emitter::{
        gen_transfer_txn_request, generate_invalid_transaction, stats::StatsAccumulator,
        wait_for_accounts_sequence, MAX_TXN_BATCH_SIZE, SEND_AMOUNT, TXN_EXPIRATION_SECONDS,
    },
    EmitThreadParams,
};
use aptos_logger::{debug, info, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use core::{
    cmp::{max, min},
    result::Result::{Err, Ok},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use rand::seq::{IteratorRandom, SliceRandom};
use std::{sync::Arc, time::Instant};
use tokio::time::sleep;

#[derive(Debug)]
pub struct SubmissionWorker {
    pub(crate) accounts: Vec<LocalAccount>,
    client: RestClient,
    all_addresses: Arc<Vec<AccountAddress>>,
    stop: Arc<AtomicBool>,
    params: EmitThreadParams,
    stats: Arc<StatsAccumulator>,
    txn_factory: TransactionFactory,
    invalid_transaction_ratio: usize,
    rng: ::rand::rngs::StdRng,
}

// Note, there is an edge case that can occur if the transaction emitter
// bursts the target node too fast, and the emitter doesn't handle it
// very well, instead waiting up until the timeout for the target seqnum
// to progress, even though it never will. See more here:
// https://github.com/aptos-labs/aptos-core/issues/1565
impl SubmissionWorker {
    pub fn new(
        accounts: Vec<LocalAccount>,
        client: RestClient,
        all_addresses: Arc<Vec<AccountAddress>>,
        stop: Arc<AtomicBool>,
        params: EmitThreadParams,
        stats: Arc<StatsAccumulator>,
        txn_factory: TransactionFactory,
        invalid_transaction_ratio: usize,
        rng: ::rand::rngs::StdRng,
    ) -> Self {
        Self {
            accounts,
            client,
            all_addresses,
            stop,
            params,
            stats,
            txn_factory,
            invalid_transaction_ratio,
            rng,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub(crate) async fn run(mut self, gas_price: u64) -> Vec<LocalAccount> {
        let check_stats_at_end = self.params.check_stats_at_end && !self.params.wait_committed;
        let wait_for_accounts_sequence_timeout = Duration::from_secs(min(
            self.params.txn_expiration_time_secs,
            TXN_EXPIRATION_SECONDS,
        ));

        let wait_duration = Duration::from_millis(self.params.wait_millis);

        let start_time = Instant::now();
        let mut total_num_requests = 0;

        while !self.stop.load(Ordering::Relaxed) {
            let requests = self.gen_requests(gas_price);
            let num_requests = requests.len();
            total_num_requests += num_requests;
            let loop_start_time = Instant::now();
            let wait_until = loop_start_time + wait_duration;
            let mut txn_offset_time = 0u64;
            for request in requests {
                let cur_time = Instant::now();
                txn_offset_time += (cur_time - loop_start_time).as_millis() as u64;
                self.stats.submitted.fetch_add(1, Ordering::Relaxed);
                let resp = self.client.submit(&request).await;
                if let Err(e) = resp {
                    warn!("[{:?}] Failed to submit request: {:?}", self.client, e);
                }
            }
            if self.params.wait_committed {
                self.update_stats(
                    loop_start_time,
                    txn_offset_time,
                    num_requests,
                    false,
                    wait_for_accounts_sequence_timeout,
                )
                .await
            }
            let now = Instant::now();
            if wait_until > now {
                sleep(wait_until - now).await;
            }
        }

        // If this was a burst mode run and the user didn't specifically opt
        // out of it, update the stats for the whole run.
        if check_stats_at_end {
            debug!("Checking stats for final time at the end");
            self.update_stats(
                start_time,
                0,
                total_num_requests,
                true,
                Duration::from_millis(500),
            )
            .await
        }

        self.accounts
    }

    /// This function assumes that num_requests == num_accounts, which is
    /// precisely how gen_requests works. If this changes, this code will
    /// need to be fixed.
    ///
    /// Note, the latency values are not accurate if --check-stats-at-end
    /// is used. There is no easy way around this accurately. As such, we
    /// don't update latency at all if that flag is set.
    async fn update_stats(
        &mut self,
        start_time: Instant,
        txn_offset_time: u64,
        num_requests: usize,
        skip_latency_stats: bool,
        wait_for_accounts_sequence_timeout: Duration,
    ) {
        match wait_for_accounts_sequence(
            &self.client,
            &mut self.accounts,
            wait_for_accounts_sequence_timeout,
        )
        .await
        {
            Ok(()) => {
                let latency = (Instant::now() - start_time).as_millis() as u64
                    - txn_offset_time / num_requests as u64;
                self.stats
                    .committed
                    .fetch_add(num_requests as u64, Ordering::Relaxed);
                if !skip_latency_stats {
                    self.stats
                        .latency
                        .fetch_add(latency * num_requests as u64, Ordering::Relaxed);
                    self.stats
                        .latencies
                        .record_data_point(latency, num_requests as u64);
                }
            }
            Err(uncommitted) => {
                let num_uncommitted = uncommitted.len() as u64;
                let num_committed = num_requests as u64 - num_uncommitted;
                // To avoid negative result caused by uncommitted tx occur
                // Simplified from:
                // end_time * num_committed - (txn_offset_time/num_requests) * num_committed
                // to
                // (end_time - txn_offset_time / num_requests) * num_committed
                let latency = (Instant::now() - start_time).as_millis() as u64
                    - txn_offset_time / num_requests as u64;
                let committed_latency = latency * num_committed as u64;
                self.stats
                    .committed
                    .fetch_add(num_committed, Ordering::Relaxed);
                self.stats
                    .expired
                    .fetch_add(num_uncommitted, Ordering::Relaxed);
                if !skip_latency_stats {
                    self.stats
                        .latency
                        .fetch_add(committed_latency, Ordering::Relaxed);
                    self.stats
                        .latencies
                        .record_data_point(latency, num_committed);
                }
                info!(
                    "[{:?}] Transactions were not committed before expiration: {:?}",
                    self.client, uncommitted
                );
            }
        }
    }

    fn gen_requests(&mut self, gas_price: u64) -> Vec<SignedTransaction> {
        let batch_size = max(MAX_TXN_BATCH_SIZE, self.accounts.len());
        let accounts = self
            .accounts
            .iter_mut()
            .choose_multiple(&mut self.rng, batch_size);
        let mut requests = Vec::with_capacity(accounts.len());
        let invalid_size = if self.invalid_transaction_ratio != 0 {
            // if enable mix invalid tx, at least 1 invalid tx per batch
            max(1, accounts.len() * self.invalid_transaction_ratio / 100)
        } else {
            0
        };
        let mut num_valid_tx = accounts.len() - invalid_size;
        for sender in accounts {
            let receiver = self
                .all_addresses
                .choose(&mut self.rng)
                .expect("all_addresses can't be empty");
            let request = if num_valid_tx > 0 {
                num_valid_tx -= 1;
                gen_transfer_txn_request(
                    sender,
                    receiver,
                    SEND_AMOUNT,
                    &self.txn_factory,
                    gas_price,
                )
            } else {
                generate_invalid_transaction(
                    sender,
                    receiver,
                    SEND_AMOUNT,
                    &self.txn_factory,
                    gas_price,
                    &requests,
                    &mut self.rng,
                )
            };
            requests.push(request);
        }
        requests
    }
}
