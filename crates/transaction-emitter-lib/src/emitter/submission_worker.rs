// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    emitter::{stats::StatsAccumulator, wait_for_accounts_sequence},
    transaction_generator::TransactionGenerator,
    EmitModeParams,
};
use aptos_logger::sample::Sampling;
use aptos_logger::{sample, sample::SampleRate, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::{transaction::SignedTransaction, LocalAccount},
};
use core::{
    cmp::{max, min},
    result::Result::{Err, Ok},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use futures::future::try_join_all;
use rand::seq::IteratorRandom;
use rand::Rng;
use std::sync::atomic::AtomicU64;
use std::{sync::Arc, time::Instant};
use tokio::time::sleep;

#[derive(Debug)]
pub struct SubmissionWorker {
    pub(crate) accounts: Vec<LocalAccount>,
    client: RestClient,
    all_addresses: Arc<Vec<AccountAddress>>,
    stop: Arc<AtomicBool>,
    params: EmitModeParams,
    stats: Arc<StatsAccumulator>,
    txn_generator: Box<dyn TransactionGenerator>,
    invalid_transaction_ratio: usize,
    worker_index: usize,
    check_account_sequence_only_once: bool,
    rng: ::rand::rngs::StdRng,
}

impl SubmissionWorker {
    pub fn new(
        accounts: Vec<LocalAccount>,
        client: RestClient,
        all_addresses: Arc<Vec<AccountAddress>>,
        stop: Arc<AtomicBool>,
        params: EmitModeParams,
        stats: Arc<StatsAccumulator>,
        txn_generator: Box<dyn TransactionGenerator>,
        invalid_transaction_ratio: usize,
        worker_index: usize,
        check_account_sequence_only_once: bool,
        rng: ::rand::rngs::StdRng,
    ) -> Self {
        Self {
            accounts,
            client,
            all_addresses,
            stop,
            params,
            stats,
            txn_generator,
            invalid_transaction_ratio,
            worker_index,
            check_account_sequence_only_once,
            rng,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub(crate) async fn run(mut self, gas_price: u64) -> Vec<LocalAccount> {
        // Introduce a random jitter between, so that:
        //  - we don't hammer the rest APIs all at once.
        //  - allow for even spread for fixed TPS setup
        let start_sleep_duration = self.start_sleep_time();
        let start_time = Instant::now() + start_sleep_duration;

        self.sleep_check_done(start_sleep_duration).await;

        let wait_duration = Duration::from_millis(self.params.wait_millis);
        let wait_for_accounts_sequence_timeout =
            Duration::from_secs(self.params.txn_expiration_time_secs + 30);
        let mut wait_until = start_time;

        while !self.stop.load(Ordering::Relaxed) {
            let loop_start_time = Arc::new(Instant::now());
            if loop_start_time.duration_since(wait_until) > wait_duration {
                warn!(
                    "[{:?}] txn_emitter worker drifted out of sync too much: {}s",
                    self.client.show_base_url_string(),
                    loop_start_time.duration_since(wait_until).as_secs()
                );
            }
            // always add expected cycle duration, to not drift from expected pace.
            wait_until += wait_duration;

            let requests = self.gen_requests(gas_price);
            let num_requests = requests.len();
            let txn_offset_time = Arc::new(AtomicU64::new(0));

            if let Err(e) = try_join_all(requests.chunks(self.params.max_submit_batch_size).map(
                |reqs| {
                    submit_transactions(
                        &self.client,
                        reqs,
                        loop_start_time.clone(),
                        txn_offset_time.clone(),
                        self.stats.clone(),
                    )
                },
            ))
            .await
            {
                sample!(
                    SampleRate::Duration(Duration::from_secs(120)),
                    warn!(
                        "[{:?}] Failed to submit request: {:?}",
                        self.client.show_base_url_string(),
                        e
                    )
                );
            }

            if self.check_account_sequence_only_once {
                self.sleep_check_done(Duration::from_secs(self.params.txn_expiration_time_secs))
                    .await;
            }

            self.update_stats(
                *loop_start_time,
                txn_offset_time.load(Ordering::Relaxed),
                num_requests,
                // skip latency if checking seq_num only once
                self.check_account_sequence_only_once,
                wait_for_accounts_sequence_timeout,
                self.check_account_sequence_only_once,
            )
            .await;

            let now = Instant::now();
            if wait_until > now {
                self.sleep_check_done(wait_until - now).await;
            }
        }

        self.accounts
    }

    async fn sleep_check_done(&self, duration: Duration) -> bool {
        let start_time = Instant::now();
        loop {
            sleep(Duration::from_secs(1)).await;
            if self.stop.load(Ordering::Relaxed) {
                return false;
            }
            if start_time.elapsed() >= duration {
                return true;
            }
        }
    }

    fn start_sleep_time(&mut self) -> Duration {
        let random_jitter_millis = if self.params.start_jitter_millis > 0 {
            self.rng.gen_range(0, self.params.start_jitter_millis)
        } else {
            0
        };
        Duration::from_millis(
            (self.params.start_offset_multiplier_millis * self.worker_index as f64) as u64
                + random_jitter_millis,
        )
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
        check_account_sequence_only_once: bool,
    ) {
        assert_eq!(
            num_requests,
            self.params.transactions_per_account * self.accounts.len()
        );
        let (num_expired, sum_of_completion_timestamps_millis) = wait_for_accounts_sequence(
            start_time,
            &self.client,
            &mut self.accounts,
            self.params.transactions_per_account,
            wait_for_accounts_sequence_timeout,
            check_account_sequence_only_once,
            &mut self.rng,
        )
        .await;

        let num_committed = num_requests - num_expired;
        // To avoid negative result caused by uncommitted tx occur
        // Simplified from:
        // sum_of_completion_timestamps_millis - (txn_offset_time/num_requests) * num_committed
        // to
        // (end_time - txn_offset_time / num_requests) * num_committed
        //
        // This approximates start time of only committed transaction to be
        // average start time of all submitted transactions.

        if num_expired > 0 {
            self.stats
                .expired
                .fetch_add(num_expired as u64, Ordering::Relaxed);
            sample!(
                SampleRate::Duration(Duration::from_secs(120)),
                warn!(
                    "[{:?}] Transactions were not committed before expiration: {:?}",
                    self.client.show_base_url_string(),
                    num_expired
                )
            );
        }

        if num_committed > 0 {
            let sum_latency = sum_of_completion_timestamps_millis
                - (txn_offset_time as u128 * num_committed as u128) / num_requests as u128;
            let avg_latency = (sum_latency / num_committed as u128) as u64;
            self.stats
                .committed
                .fetch_add(num_committed as u64, Ordering::Relaxed);

            if !skip_latency_stats {
                self.stats
                    .latency
                    .fetch_add(sum_latency as u64, Ordering::Relaxed);
                self.stats
                    .latency_samples
                    .fetch_add(num_committed as u64, Ordering::Relaxed);
                self.stats
                    .latencies
                    .record_data_point(avg_latency, num_committed as u64);
            }
        }
    }

    fn gen_requests(&mut self, gas_price: u64) -> Vec<SignedTransaction> {
        let batch_size = max(
            1,
            min(
                self.params.max_submit_batch_size / self.params.transactions_per_account,
                self.accounts.len(),
            ),
        );
        let accounts = self
            .accounts
            .iter_mut()
            .choose_multiple(&mut self.rng, batch_size);
        self.txn_generator.generate_transactions(
            accounts,
            self.params.transactions_per_account,
            self.all_addresses.clone(),
            self.invalid_transaction_ratio,
            gas_price,
        )
    }
}

pub async fn submit_transactions(
    client: &RestClient,
    txns: &[SignedTransaction],
    loop_start_time: Arc<Instant>,
    txn_offset_time: Arc<AtomicU64>,
    stats: Arc<StatsAccumulator>,
) -> anyhow::Result<()> {
    let cur_time = Instant::now();
    let offset = cur_time - *loop_start_time;
    txn_offset_time.fetch_add(
        txns.len() as u64 * offset.as_millis() as u64,
        Ordering::Relaxed,
    );
    stats
        .submitted
        .fetch_add(txns.len() as u64, Ordering::Relaxed);

    match client.submit_batch_bcs(txns).await {
        Err(e) => sample!(
            SampleRate::Duration(Duration::from_secs(5)),
            warn!(
                "[{:?}] Failed to submit batch request: {:?}",
                client.show_base_url_string(),
                e
            )
        ),
        Ok(v) => {
            for f in v.into_inner().transaction_failures {
                sample!(
                    SampleRate::Duration(Duration::from_secs(5)),
                    warn!(
                        "[{:?}] Failed to submit a request within a batch: {:?}",
                        client.show_base_url_string(),
                        f
                    )
                );
            }
        }
    };
    Ok(())
}
