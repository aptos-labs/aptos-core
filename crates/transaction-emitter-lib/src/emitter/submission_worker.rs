// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    emitter::{
        stats::{DynamicStatsTracking, StatsAccumulator},
        update_seq_num_and_get_num_expired, wait_for_accounts_sequence,
    },
    EmitModeParams,
};
use aptos_logger::{sample, sample::SampleRate, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::{transaction::SignedTransaction, vm_status::StatusCode, LocalAccount},
};
use aptos_transaction_generator_lib::TransactionGenerator;
use core::{
    cmp::{max, min},
    result::Result::{Err, Ok},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use futures::future::join_all;
use itertools::Itertools;
use rand::seq::IteratorRandom;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
    time::Instant,
};
use tokio::time::sleep;

pub struct SubmissionWorker {
    pub(crate) accounts: Vec<LocalAccount>,
    client: RestClient,
    stop: Arc<AtomicBool>,
    params: EmitModeParams,
    stats: Arc<DynamicStatsTracking>,
    txn_generator: Box<dyn TransactionGenerator>,
    start_sleep_duration: Duration,
    skip_latency_stats: bool,
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
        start_sleep_duration: Duration,
        skip_latency_stats: bool,
        rng: ::rand::rngs::StdRng,
    ) -> Self {
        Self {
            accounts,
            client,
            stop,
            params,
            stats,
            txn_generator,
            start_sleep_duration,
            skip_latency_stats,
            rng,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub(crate) async fn run(mut self) -> Vec<LocalAccount> {
        let start_time = Instant::now() + self.start_sleep_duration;

        self.sleep_check_done(self.start_sleep_duration).await;

        let wait_duration = Duration::from_millis(self.params.wait_millis);
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

            let mut account_to_start_and_end_seq_num = HashMap::new();
            for req in requests.iter() {
                let cur = req.sequence_number();
                let _ = *account_to_start_and_end_seq_num
                    .entry(req.sender())
                    .and_modify(|(start, end)| {
                        if *start > cur {
                            *start = cur;
                        }
                        if *end < cur + 1 {
                            *end = cur + 1;
                        }
                    })
                    .or_insert((cur, cur + 1));
            }

            let txn_expiration_time = requests
                .iter()
                .map(|txn| txn.expiration_timestamp_secs())
                .max()
                .unwrap_or(0);

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

            if self.skip_latency_stats {
                // we also don't want to be stuck waiting for txn_expiration_time_secs
                // after stop is called, so we sleep until time or stop is set.
                self.sleep_check_done(Duration::from_secs(
                    self.params.txn_expiration_time_secs + 20,
                ))
                .await
            }

            self.wait_and_update_stats(
                *loop_start_time,
                txn_offset_time.load(Ordering::Relaxed) / (requests.len() as u64),
                account_to_start_and_end_seq_num,
                // skip latency if asked to check seq_num only once
                // even if we check more often due to stop (to not affect sampling)
                self.skip_latency_stats,
                txn_expiration_time,
                // if we don't care about latency, we can recheck less often.
                // generally, we should never need to recheck, as we wait enough time
                // before calling here, but in case of shutdown/or client we are talking
                // to being stale (having stale transaction_version), we might need to wait.
                Duration::from_millis(
                    if self.skip_latency_stats { 10 } else { 1 }
                        * self.params.check_account_sequence_sleep_millis,
                ),
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
    async fn sleep_check_done(&self, duration: Duration) {
        let start_time = Instant::now();
        loop {
            sleep(Duration::from_secs(1)).await;
            if self.stop.load(Ordering::Relaxed) {
                return;
            }
            if start_time.elapsed() >= duration {
                return;
            }
        }
    }

    /// This function assumes that num_requests == num_accounts, which is
    /// precisely how gen_requests works. If this changes, this code will
    /// need to be fixed.
    ///
    /// Note, the latency values are not accurate if --check-stats-at-end
    /// is used. There is no easy way around this accurately. As such, we
    /// don't update latency at all if that flag is set.
    async fn wait_and_update_stats(
        &mut self,
        start_time: Instant,
        avg_txn_offset_time: u64,
        account_to_start_and_end_seq_num: HashMap<AccountAddress, (u64, u64)>,
        skip_latency_stats: bool,
        txn_expiration_ts_secs: u64,
        check_account_sleep_duration: Duration,
        loop_stats: &StatsAccumulator,
    ) {
        let (latest_fetched_counts, sum_of_completion_timestamps_millis) =
            wait_for_accounts_sequence(
                start_time,
                &self.client,
                &account_to_start_and_end_seq_num,
                txn_expiration_ts_secs,
                check_account_sleep_duration,
            )
            .await;

        let (num_committed, num_expired) = update_seq_num_and_get_num_expired(
            &mut self.accounts,
            account_to_start_and_end_seq_num,
            latest_fetched_counts,
        );

        if num_expired > 0 {
            loop_stats
                .expired
                .fetch_add(num_expired as u64, Ordering::Relaxed);
            sample!(
                SampleRate::Duration(Duration::from_secs(120)),
                warn!(
                    "[{:?}] Transactions were not committed before expiration: {:?}, for {:?}",
                    self.client.path_prefix_string(),
                    num_expired,
                    self.accounts
                        .iter()
                        .map(|a| a.address())
                        .collect::<Vec<_>>(),
                )
            );
        }

        if num_committed > 0 {
            let sum_latency = sum_of_completion_timestamps_millis
                - (avg_txn_offset_time as u128 * num_committed as u128);
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
                SampleRate::Duration(Duration::from_secs(60)),
                warn!(
                    "[{:?}] Failed to submit batch request: {:?}",
                    client.path_prefix_string(),
                    e
                )
            );
        },
        Ok(v) => {
            let failures = v.into_inner().transaction_failures;

            stats
                .failed_submission
                .fetch_add(failures.len() as u64, Ordering::Relaxed);

            sample!(SampleRate::Duration(Duration::from_secs(60)), {
                let by_error = failures
                    .iter()
                    .map(|f| {
                        f.error
                            .vm_error_code
                            .and_then(|c| StatusCode::try_from(c).ok())
                    })
                    .counts();
                if let Some(failure) = failures.first() {
                    let sender = txns[failure.transaction_index].sender();

                    let last_transactions =
                        if let Ok(account) = client.get_account_bcs(sender).await {
                            client
                                .get_account_transactions_bcs(
                                    sender,
                                    Some(account.into_inner().sequence_number().saturating_sub(1)),
                                    Some(5),
                                )
                                .await
                                .ok()
                                .map(|r| r.into_inner())
                        } else {
                            None
                        };
                    let balance = client
                        .get_account_balance(sender)
                        .await
                        .map_or(-1, |v| v.into_inner().get() as i64);

                    warn!(
                        "[{:?}] Failed to submit {} txns in a batch, first failure due to {:?}, for account {}, first asked: {}, failed seq nums: {:?}, failed error codes: {:?}, balance of {} and last transaction for account: {:?}",
                        client.path_prefix_string(),
                        failures.len(),
                        failure,
                        sender,
                        txns[0].sequence_number(),
                        failures.iter().map(|f| txns[f.transaction_index].sequence_number()).collect::<Vec<_>>(),
                        by_error,
                        balance,
                        last_transactions,
                    );
                }
            });
        },
    };
}
