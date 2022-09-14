// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    emitter::{
        stats::{DynamicStatsTracking, StatsAccumulator},
        wait_for_accounts_sequence,
    },
    transaction_generator::TransactionGenerator,
    EmitModeParams,
};
use aptos_logger::sample::Sampling;
use aptos_logger::{sample, sample::SampleRate, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::{transaction::SignedTransaction, vm_status::StatusCode, LocalAccount};
use core::{
    cmp::{max, min},
    result::Result::{Err, Ok},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use futures::future::join_all;
use rand::seq::IteratorRandom;
use rand::Rng;
use std::sync::atomic::AtomicU64;
use std::{sync::Arc, time::Instant};
use tokio::time::sleep;

pub struct SubmissionWorker {
    pub(crate) accounts: Vec<LocalAccount>,
    client: RestClient,
    stop: Arc<AtomicBool>,
    params: EmitModeParams,
    stats: Arc<DynamicStatsTracking>,
    txn_generator: Box<dyn TransactionGenerator>,
    worker_index: usize,
    check_account_sequence_only_once: bool,
    rng: ::rand::rngs::StdRng,
}

impl SubmissionWorker {
    pub fn new(
        accounts: Vec<LocalAccount>,
        client: RestClient,
        stop: Arc<AtomicBool>,
        params: EmitModeParams,
        stats: Arc<DynamicStatsTracking>,
        txn_generator: Box<dyn TransactionGenerator>,
        worker_index: usize,
        check_account_sequence_only_once: bool,
        rng: ::rand::rngs::StdRng,
    ) -> Self {
        Self {
            accounts,
            client,
            stop,
            params,
            stats,
            txn_generator,
            worker_index,
            check_account_sequence_only_once,
            rng,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub(crate) async fn run(mut self) -> Vec<LocalAccount> {
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
            let stats_clone = self.stats.clone();
            let loop_stats = stats_clone.get_cur();

            let loop_start_time = Arc::new(Instant::now());
            if wait_duration.as_secs() > 0
                && loop_start_time.duration_since(wait_until) > wait_duration
            {
                sample!(
                    SampleRate::Duration(Duration::from_secs(120)),
                    warn!(
                        "[{:?}] txn_emitter worker drifted out of sync too much: {}s",
                        self.client.path_prefix_string(),
                        loop_start_time.duration_since(wait_until).as_secs()
                    )
                );
            }
            // always add expected cycle duration, to not drift from expected pace.
            wait_until += wait_duration;

            let requests = self.gen_requests();
            let num_requests = requests.len();
            let txn_offset_time = Arc::new(AtomicU64::new(0));

            join_all(
                requests
                    .chunks(self.params.max_submit_batch_size)
                    .map(|reqs| {
                        submit_transactions(
                            &self.client,
                            reqs,
                            loop_start_time.clone(),
                            txn_offset_time.clone(),
                            loop_stats,
                        )
                    }),
            )
            .await;

            let early_return_due_to_stop = if self.check_account_sequence_only_once {
                // we also don't want to be stuck waiting for txn_expiration_time_secs
                // after stop is called, so we sleep until time or stop is set.
                self.sleep_check_done(Duration::from_secs(self.params.txn_expiration_time_secs))
                    .await
            } else {
                false
            };

            self.update_stats(
                *loop_start_time,
                txn_offset_time.load(Ordering::Relaxed),
                num_requests,
                // skip latency if asked to check seq_num only once
                // even if we check more often due to stop (to not affect sampling)
                self.check_account_sequence_only_once,
                wait_for_accounts_sequence_timeout,
                // if we needed to stop sleep early, we should check until complete
                // as not enough time might have passed otherwise for txn to be committed.
                self.check_account_sequence_only_once && !early_return_due_to_stop,
                loop_stats,
            )
            .await;

            let now = Instant::now();
            if wait_until > now {
                self.sleep_check_done(wait_until - now).await;
            }
        }

        self.accounts
    }

    // returns true if it returned early
    async fn sleep_check_done(&self, duration: Duration) -> bool {
        let start_time = Instant::now();
        loop {
            sleep(Duration::from_secs(1)).await;
            if self.stop.load(Ordering::Relaxed) {
                return true;
            }
            if start_time.elapsed() >= duration {
                return false;
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
        loop_stats: &StatsAccumulator,
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
            Duration::from_millis(self.params.check_account_sequence_sleep_millis),
        )
        .await;

        let num_committed = num_requests - num_expired;

        if num_expired > 0 {
            loop_stats
                .expired
                .fetch_add(num_expired as u64, Ordering::Relaxed);
            sample!(
                SampleRate::Duration(Duration::from_secs(120)),
                warn!(
                    "[{:?}] Transactions were not committed before expiration: {:?}",
                    self.client.path_prefix_string(),
                    num_expired
                )
            );
        }

        if num_committed > 0 {
            let sum_latency = sum_of_completion_timestamps_millis
                - (txn_offset_time as u128 * num_committed as u128) / num_requests as u128;
            let avg_latency = (sum_latency / num_committed as u128) as u64;
            loop_stats
                .committed
                .fetch_add(num_committed as u64, Ordering::Relaxed);

            if !skip_latency_stats {
                loop_stats
                    .latency
                    .fetch_add(sum_latency as u64, Ordering::Relaxed);
                loop_stats
                    .latency_samples
                    .fetch_add(num_committed as u64, Ordering::Relaxed);
                loop_stats
                    .latencies
                    .record_data_point(avg_latency, num_committed as u64);
            }
        }
    }

    fn gen_requests(&mut self) -> Vec<SignedTransaction> {
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
        self.txn_generator
            .generate_transactions(accounts, self.params.transactions_per_account)
    }
}

pub async fn submit_transactions(
    client: &RestClient,
    txns: &[SignedTransaction],
    loop_start_time: Arc<Instant>,
    txn_offset_time: Arc<AtomicU64>,
    stats: &StatsAccumulator,
) {
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
        Err(e) => {
            stats
                .failed_submission
                .fetch_add(txns.len() as u64, Ordering::Relaxed);
            sample!(
                SampleRate::Duration(Duration::from_secs(120)),
                warn!(
                    "[{:?}] Failed to submit batch request: {:?}",
                    client.path_prefix_string(),
                    e
                )
            );
        }
        Ok(v) => {
            let failures = v.into_inner().transaction_failures;
            stats
                .failed_submission
                .fetch_add(failures.len() as u64, Ordering::Relaxed);

            let too_old_failures = failures
                .iter()
                .filter(|f| {
                    Some(u64::from(StatusCode::SEQUENCE_NUMBER_TOO_OLD)) == f.error.vm_error_code
                })
                .collect::<Vec<_>>();
            if let Some(f) = too_old_failures.first() {
                let txn = &txns[f.transaction_index];
                if let Ok(account) = client.get_account(txn.sender()).await {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(15)),
                        warn!(
                            "[{:?}] Failed to submit due to SEQUENCE_NUMBER_TOO_OLD, current: {}, first asked: {}, failed: {:?}",
                            client.path_prefix_string(),
                            account.into_inner().sequence_number,
                            txns[0].sequence_number(),
                            too_old_failures.iter().map(|f| txns[f.transaction_index].sequence_number()).collect::<Vec<_>>(),
                        )
                    );
                }
            }

            for f in failures {
                sample!(
                    SampleRate::Duration(Duration::from_secs(120)),
                    warn!(
                        "[{:?}] Failed to submit a request within a batch: {:?}",
                        client.path_prefix_string(),
                        f
                    )
                );
            }
        }
    };
}
