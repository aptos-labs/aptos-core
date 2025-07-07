// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    emitter::stats::{DynamicStatsTracking, StatsAccumulator},
    query_sequence_numbers, EmitModeParams,
};
use aptos_logger::{sample, sample::SampleRate};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::{transaction::SignedTransaction, vm_status::StatusCode, LocalAccount},
};
use aptos_transaction_generator_lib::TransactionGenerator;
use aptos_types::transaction::ReplayProtector;
use core::{
    cmp::{max, min},
    result::Result::{Err, Ok},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use futures::future::join_all;
use itertools::Itertools;
use log::{debug, error, info, warn};
use rand::seq::IteratorRandom;
use std::{
    borrow::Borrow,
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
    time::Instant,
};
use tokio::time::{sleep, sleep_until};

const ALLOWED_EARLY: Duration = Duration::from_micros(500);

use super::query_txn_summaries;

pub struct AccountData {
    submitted_sequence_number_range: Option<(u64, u64)>,
    // For each submitted transaction, we store the replay protector, (submission time, expiration time)
    submitted_replay_protectors: HashMap<ReplayProtector, (Instant, u64)>,
    version_to_fetch_next: u64,
}
pub struct SubmissionWorker {
    pub(crate) accounts: Vec<Arc<LocalAccount>>,
    clients: Arc<Vec<RestClient>>,
    /// Main one is used to submit requests, all are used for querying/latency
    main_client_index: usize,
    stop: Arc<AtomicBool>,
    params: EmitModeParams,
    stats: Arc<DynamicStatsTracking>,
    txn_generator: Box<dyn TransactionGenerator>,
    start_sleep_duration: Duration,
    skip_latency_stats: bool,
    rng: ::rand::rngs::StdRng,
    account_data: HashMap<AccountAddress, AccountData>,
}

impl SubmissionWorker {
    pub fn new(
        accounts: Vec<LocalAccount>,
        clients: Arc<Vec<RestClient>>,
        main_client_index: usize,
        stop: Arc<AtomicBool>,
        params: EmitModeParams,
        stats: Arc<DynamicStatsTracking>,
        txn_generator: Box<dyn TransactionGenerator>,
        start_sleep_duration: Duration,
        skip_latency_stats: bool,
        rng: ::rand::rngs::StdRng,
    ) -> Self {
        let accounts = accounts.into_iter().map(Arc::new).collect();
        Self {
            accounts,
            clients,
            main_client_index,
            stop,
            params,
            stats,
            txn_generator,
            start_sleep_duration,
            skip_latency_stats,
            rng,
            account_data: HashMap::new(),
        }
    }

    fn client(&self) -> &RestClient {
        &self.clients[self.main_client_index]
    }

    #[allow(clippy::collapsible_if)]
    pub(crate) async fn run(mut self, start_instant: Instant) -> Vec<LocalAccount> {
        let mut wait_until = start_instant + self.start_sleep_duration;

        self.sleep_check_done(wait_until).await;
        let wait_duration = Duration::from_millis(self.params.wait_millis);

        while !self.stop.load(Ordering::Relaxed) {
            let loop_start_time = Instant::now();

            if wait_duration.as_secs() > 0 {
                self.verify_loop_start_drift(loop_start_time, wait_until);
            }

            let stats_clone = self.stats.clone();
            let loop_stats = stats_clone.get_cur();

            let requests = self.gen_requests();
            if !requests.is_empty() {
                for req in requests.iter() {
                    let cur =
                        self.account_data
                            .entry(req.sender())
                            .or_insert_with(|| AccountData {
                                submitted_sequence_number_range: None,
                                submitted_replay_protectors: HashMap::new(),
                                version_to_fetch_next: 0,
                            });
                    cur.submitted_replay_protectors.insert(
                        req.replay_protector(),
                        (Instant::now(), req.expiration_timestamp_secs()),
                    );
                    println!(
                        "replay_protector {:?} expiration timestamp: {:?}",
                        req.replay_protector(),
                        req.expiration_timestamp_secs()
                            - aptos_infallible::duration_since_epoch().as_secs()
                    );
                    // info!(
                    //     "(address: {:?}, replay_protector: {:?}, expiration_timestamp_secs: {:?})",
                    //     req.sender(),
                    //     req.replay_protector(),
                    //     req.expiration_timestamp_secs()
                    // );
                    match req.replay_protector() {
                        ReplayProtector::SequenceNumber(seq_num) => {
                            if cur.submitted_sequence_number_range.is_none() {
                                cur.submitted_sequence_number_range = Some((seq_num, seq_num + 1));
                            } else {
                                let (start, end) =
                                    cur.submitted_sequence_number_range.as_mut().unwrap();
                                if *start > seq_num {
                                    *start = seq_num;
                                }
                                if *end < seq_num + 1 {
                                    *end = seq_num + 1;
                                }
                            }
                        },
                        ReplayProtector::Nonce(_) => {},
                    }
                }
                // Some transaction generators use burner accounts, and will have different
                // number of accounts per transaction, so useful to very rarely log.
                sample!(
                    SampleRate::Duration(Duration::from_secs(300)),
                    info!(
                        "[{:?}] txn_emitter worker: handling {} accounts, generated txns for: {}",
                        self.client().path_prefix_string(),
                        self.accounts.len(),
                        self.account_data.len(),
                    )
                );

                // let txn_expiration_time = requests
                //     .iter()
                //     .map(|txn| txn.expiration_timestamp_secs())
                //     .max()
                //     .unwrap_or(0);

                let txn_offset_time = Arc::new(AtomicU64::new(0));

                join_all(
                    requests
                        .chunks(self.params.max_submit_batch_size)
                        .map(|reqs| {
                            submit_transactions(
                                self.client(),
                                reqs,
                                loop_start_time,
                                txn_offset_time.clone(),
                                loop_stats,
                            )
                        }),
                )
                .await;

                let submitted_after = loop_start_time.elapsed();
                if submitted_after.as_secs() > 5 {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(30)),
                        warn!(
                            "[{:?}] txn_emitter worker waited for more than 5s to submit transactions: {}s after loop start",
                            self.client().path_prefix_string(),
                            submitted_after.as_secs(),
                        )
                    );
                }

                if self.skip_latency_stats {
                    // we also don't want to be stuck waiting for txn_expiration_time_secs
                    // after stop is called, so we sleep until time or stop is set.
                    self.sleep_check_done(
                        Instant::now()
                            + Duration::from_secs(self.params.txn_expiration_time_secs + 3),
                    )
                    .await
                }

                self.wait_and_update_stats(
                    txn_offset_time.load(Ordering::Relaxed) / (requests.len() as u64),
                    // skip latency if asked to check seq_num only once
                    // even if we check more often due to stop (to not affect sampling)
                    self.skip_latency_stats,
                    // if we don't care about latency, we can recheck less often.
                    // generally, we should never need to recheck, as we wait enough time
                    // before calling here, but in case of shutdown/or client we are talking
                    // to being stale (having stale transaction_version), we might need to wait.
                    if self.skip_latency_stats {
                        (10 * self.params.check_account_sequence_sleep).max(Duration::from_secs(3))
                    } else {
                        self.params.check_account_sequence_sleep
                    },
                    loop_stats,
                )
                .await;
            }

            if wait_duration.as_secs() > 0 {
                // always add expected cycle duration, to not drift from expected pace,
                // irrespectively of how long our iteration lasted.
                wait_until += wait_duration;
                self.sleep_check_done(wait_until).await;
            }
        }

        self.accounts
            .into_iter()
            .map(|account_arc_mutex| Arc::into_inner(account_arc_mutex).unwrap())
            .collect()
    }

    // returns true if it returned early
    async fn sleep_check_done(&self, sleep_until_time: Instant) {
        // sleep has millisecond granularity - so round the sleep
        let sleep_poll_interval = Duration::from_secs(1);
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return;
            }

            let now = Instant::now();
            if now + ALLOWED_EARLY > sleep_until_time {
                return;
            }

            if sleep_until_time > now + sleep_poll_interval {
                sleep(sleep_poll_interval).await;
            } else {
                sleep_until(sleep_until_time.into()).await;
            }
        }
    }

    fn verify_loop_start_drift(&self, loop_start_time: Instant, wait_until: Instant) {
        if loop_start_time > wait_until {
            let delay_s = loop_start_time
                .saturating_duration_since(wait_until)
                .as_secs_f32();
            if delay_s > 5.0 {
                sample!(
                    SampleRate::Duration(Duration::from_secs(2)),
                    error!(
                        "[{:?}] txn_emitter worker drifted out of sync too much: {:.3}s. Is machine underprovisioned? Is expiration too short, or 5s buffer on top of it?",
                        self.client().path_prefix_string(),
                        delay_s,
                    )
                );
            } else if delay_s > 0.3 {
                sample!(
                    SampleRate::Duration(Duration::from_secs(5)),
                    error!(
                        "[{:?}] txn_emitter worker called a bit out of sync: {:.3}s. Is machine underprovisioned? Is expiration too short, or 5s buffer on top of it?",
                        self.client().path_prefix_string(),
                        delay_s,
                    )
                );
            }
        } else {
            let early_s = wait_until.saturating_duration_since(loop_start_time);
            if early_s > ALLOWED_EARLY {
                sample!(
                    SampleRate::Duration(Duration::from_secs(5)),
                    error!(
                        "[{:?}] txn_emitter worker called too early: {:.3}s. There is some bug in waiting.",
                        self.client().path_prefix_string(),
                        early_s.as_secs_f32(),
                    )
                );
            }
        }
    }

    async fn wait_for_account_txn_summaries(
        &mut self,
        sleep_between_cycles: Duration,
    ) -> (usize, usize, u128) {
        let mut sum_of_completion_timestamps_millis = 0;
        let mut num_committed = 0;
        let mut num_expired = 0;
        let mut counter = 0;
        let start = Instant::now();
        loop {
            let loop_start_time = Instant::now();
            counter += 1;
            let start_version_by_address: Vec<_> = self
                .account_data
                .iter()
                .flat_map(|(address, data)| {
                    // Fetch txn summary for the account only if there are some outstanding submitted transactions from the account
                    if !data.submitted_replay_protectors.is_empty() {
                        Some((*address, data.version_to_fetch_next))
                    } else {
                        None
                    }
                })
                .collect();
            match query_txn_summaries(self.client(), start_version_by_address.clone()).await {
                Ok((account_to_txn_summaries, ledger_timestamp)) => {
                    // Remove committed transactions from self.account_data
                    for (account, txn_summaries) in account_to_txn_summaries {
                        for txn_summary in txn_summaries {
                            // ensure!(txn_summary.sender == account, "Received transaction summary for wrong account");
                            if let Some(account_data) = self.account_data.get_mut(&account) {
                                if let Some((submitted_time, _expiration_time)) = account_data
                                    .submitted_replay_protectors
                                    .remove(&txn_summary.replay_protector())
                                {
                                    sum_of_completion_timestamps_millis +=
                                        submitted_time.elapsed().as_millis();
                                    info!("wait_for_account_txn_summaries committed replay_protector: {:?} took {:?}", txn_summary.replay_protector(), submitted_time.elapsed());
                                    num_committed += 1;
                                }
                                account_data.version_to_fetch_next = max(
                                    account_data.version_to_fetch_next,
                                    txn_summary.version() + 1,
                                );
                            }
                        }
                    }

                    // Remove expired transactions from self.account_data
                    for (account, _) in start_version_by_address {
                        if let Some(account_data) = self.account_data.get_mut(&account) {
                            account_data.submitted_replay_protectors.retain(|replay_protector, (submitted_time, expiration_time)| {
                                if ledger_timestamp > *expiration_time {
                                    num_expired += 1;
                                    info!("wait_for_account_txn_summaries expired replay_protector: {:?} took {:?}", replay_protector, submitted_time.elapsed());
                                    false // Remove the entry
                                } else {
                                    true // Keep the entry
                                }
                            });
                        }
                    }
                },
                Err(e) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(60)),
                        warn!(
                            "[{}] Failed to query txn summary on accounts {:?}: {:?}",
                            self.client().path_prefix_string(),
                            self.account_data.keys(),
                            e
                        )
                    );
                },
            }

            // All the submitted transactions are either committed or expired
            if self
                .account_data
                .iter()
                .all(|(_, data)| data.submitted_replay_protectors.is_empty())
            {
                break;
            }

            // TODO: Update sequence numbers for self.accounts. Question: Should we fetch the account resources again?

            let max_txn_expiration_ts_secs = self
                .account_data
                .values()
                .map(|data| {
                    data.submitted_replay_protectors
                        .values()
                        .map(|(_, expiration_time)| *expiration_time)
                        .max()
                        .unwrap()
                })
                .max()
                .unwrap();
            if aptos_infallible::duration_since_epoch().as_secs()
                >= max_txn_expiration_ts_secs + 240
            {
                sample!(
                    SampleRate::Duration(Duration::from_secs(15)),
                    error!(
                        "[{}] Client cannot catch up to needed timestamp ({}), after additional 240s, aborting",
                        self.client().path_prefix_string(),
                        max_txn_expiration_ts_secs,
                    )
                );
                break;
            }

            info!(
                "Sleeping for {:?}. wait_for_account_txn_summaries loop took {:?}",
                sleep_between_cycles,
                loop_start_time.elapsed()
            );
            sleep(sleep_between_cycles).await;
        }

        info!(
            "wait_for_account_txn_summaries took {} cycles, {:?}",
            counter,
            start.elapsed()
        );
        info!(
            "num_committed: {}, num_expired: {}, sum_of_completion_timestamps_millis: {}",
            num_committed, num_expired, sum_of_completion_timestamps_millis
        );
        (
            num_committed,
            num_expired,
            sum_of_completion_timestamps_millis,
        )
    }

    /// This function waits for the submitted transactions to be committed, up to
    /// a wait_timeout (counted from the start_time passed in, not from the function call).
    /// It returns number of transactions that expired without being committed,
    /// and sum of completion timestamps for those that have.
    ///
    /// This function updates sequence_number for the account to match what
    /// we were able to fetch last.
    async fn wait_for_account_sequence_numbers(
        &mut self,
        sleep_between_cycles: Duration,
    ) -> (usize, usize, u128, HashMap<AccountAddress, u64>) {
        let mut sum_of_completion_timestamps_millis = 0;
        let mut latest_fetched_counts = HashMap::new();
        let mut num_committed = 0;
        let mut num_expired = 0;
        loop {
            let pending_addresses: Vec<_> = self
                .account_data
                .iter()
                .flat_map(|(address, data)| {
                    if data.submitted_sequence_number_range.is_some() {
                        Some(*address)
                    } else {
                        None
                    }
                })
                .collect();
            match query_sequence_numbers(self.client(), pending_addresses.clone()).await {
                Ok((sequence_numbers, ledger_timestamp_secs)) => {
                    for (address, account_sequence_number) in sequence_numbers {
                        let account_data = self.account_data.get_mut(&address).unwrap();

                        account_data.submitted_replay_protectors.retain(
                            |replay_protector, (submitted_time, expiration_time)| {
                                match replay_protector {
                                    ReplayProtector::SequenceNumber(seq_num) => {
                                        if *seq_num <= account_sequence_number {
                                            sum_of_completion_timestamps_millis +=
                                                submitted_time.elapsed().as_millis();
                                            num_committed += 1;
                                            false // Remove the entry
                                        } else if *expiration_time < ledger_timestamp_secs {
                                            num_expired += 1;
                                            false // Remove the entry
                                        } else {
                                            true // Remove the entry
                                        }
                                    },
                                    ReplayProtector::Nonce(_) => {
                                        // This case shouldn't happen, as we call wait_for_txn_summaries function
                                        // when there are orderless transactions in the submitted transactions.
                                        false // Remove the entry
                                    },
                                }
                            },
                        );

                        latest_fetched_counts.insert(address, account_sequence_number);

                        if account_sequence_number
                            >= account_data.submitted_sequence_number_range.unwrap().1
                        {
                            account_data.submitted_sequence_number_range = None;
                        }
                    }

                    // All the submitted transactions are either committed or expired
                    if self
                        .account_data
                        .iter()
                        .all(|(_, data)| data.submitted_replay_protectors.is_empty())
                    {
                        break;
                    }
                },
                Err(e) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(60)),
                        warn!(
                            "[{}] Failed to query ledger info on accounts {:?}: {:?}",
                            self.client().path_prefix_string(),
                            pending_addresses,
                            e
                        )
                    );
                },
            }

            let max_txn_expiration_ts_secs = self
                .account_data
                .values()
                .map(|data| {
                    data.submitted_replay_protectors
                        .values()
                        .map(|(_, expiration_time)| *expiration_time)
                        .max()
                        .unwrap()
                })
                .max()
                .unwrap();

            if aptos_infallible::duration_since_epoch().as_secs()
                >= max_txn_expiration_ts_secs + 240
            {
                sample!(
                    SampleRate::Duration(Duration::from_secs(15)),
                    error!(
                        "[{}] Client cannot catch up to needed timestamp ({}), after additional 240s, aborting",
                        self.client().path_prefix_string(),
                        max_txn_expiration_ts_secs,
                    )
                );
                break;
            }

            sleep(sleep_between_cycles).await;
        }

        (
            num_committed,
            num_expired,
            sum_of_completion_timestamps_millis,
            latest_fetched_counts,
        )
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
        avg_txn_offset_time: u64,
        skip_latency_stats: bool,
        check_account_sleep_duration: Duration,
        loop_stats: &StatsAccumulator,
    ) {
        // If all the submitted transactions are ordered transactions, then just fetch
        // the account resource and update the sequence number.
        // If some of the submitted transactions are orderless transactions, then fetch the
        // transaction summaries instead.

        let has_submitted_orderless_txns = self.account_data.iter().any(|(_, data)| {
            data.submitted_replay_protectors
                .keys()
                .any(|rp| matches!(*rp, ReplayProtector::Nonce(_)))
        });

        let (num_committed, num_expired, sum_of_completion_timestamps_millis) =
            if has_submitted_orderless_txns {
                // Some of the submitted transactions are orderless transactions.
                let start = Instant::now();
                let (num_committed, num_expired, sum_of_completion_timestamps_millis) = self
                    .wait_for_account_txn_summaries(check_account_sleep_duration)
                    .await;
                info!("wait_for_account_txn_summaries took {:?}", start.elapsed());
                (
                    num_committed,
                    num_expired,
                    sum_of_completion_timestamps_millis,
                )
                // TODO: Need to update account sequence numbers here as well in case the account has sent sequence number based transactions.
            } else {
                let (
                    num_committed,
                    num_expired,
                    sum_of_completion_timestamps_millis,
                    latest_fetched_counts,
                ) = self
                    .wait_for_account_sequence_numbers(check_account_sleep_duration)
                    .await;

                self.update_account_seq_num(&latest_fetched_counts);
                (
                    num_committed,
                    num_expired,
                    sum_of_completion_timestamps_millis,
                )
            };

        if num_expired > 0 {
            loop_stats
                .expired
                .fetch_add(num_expired as u64, Ordering::Relaxed);
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                warn!(
                    "[{:?}] Transactions were not committed before expiration: {:?}, for {:?}",
                    self.client().path_prefix_string(),
                    num_expired,
                    self.accounts
                        .iter()
                        .map(|a| a.address())
                        .collect::<Vec<_>>(),
                )
            );
        }

        if num_committed > 0 {
            loop_stats
                .committed
                .fetch_add(num_committed as u64, Ordering::Relaxed);

            if !skip_latency_stats {
                let sum_latency = sum_of_completion_timestamps_millis
                    - (avg_txn_offset_time as u128 * num_committed as u128);
                let avg_latency = (sum_latency / num_committed as u128) as u64;
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
            .iter()
            .choose_multiple(&mut self.rng, batch_size);

        accounts
            .into_iter()
            .flat_map(|account| {
                self.txn_generator
                    .generate_transactions(account.borrow(), self.params.transactions_per_account)
            })
            .collect()
    }

    fn update_account_seq_num(&mut self, latest_fetched_counts: &HashMap<AccountAddress, u64>) {
        for account in self.accounts.iter_mut() {
            let (start_seq_num, end_seq_num) = if let Some(pair) = self
                .account_data
                .get(&account.address())
                .and_then(|data| data.submitted_sequence_number_range)
            {
                pair
            } else {
                return;
            };
            assert!(account.sequence_number() == end_seq_num);

            match latest_fetched_counts.get(&account.address()) {
                Some(count) => {
                    if *count != account.sequence_number() {
                        assert!(account.sequence_number() > *count);
                        debug!(
                            "Stale sequence_number for {}, expected {}, setting to {}",
                            account.address(),
                            account.sequence_number(),
                            count
                        );
                        account.set_sequence_number(*count);
                    }
                },
                None => {
                    debug!(
                        "Couldn't fetch sequence_number for {}, expected {}, setting to {}",
                        account.address(),
                        account.sequence_number(),
                        start_seq_num
                    );
                    account.set_sequence_number(start_seq_num);
                },
            }
        }
    }
}

pub async fn submit_transactions(
    client: &RestClient,
    txns: &[SignedTransaction],
    loop_start_time: Instant,
    txn_offset_time: Arc<AtomicU64>,
    stats: &StatsAccumulator,
) {
    let cur_time = Instant::now();
    let offset = cur_time - loop_start_time;
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

            let by_error = failures
                .iter()
                .map(|f| {
                    f.error
                        .vm_error_code
                        .and_then(|c| StatusCode::try_from(c).ok())
                })
                .counts();
            if let Some(failure) = failures.first() {
                sample!(SampleRate::Duration(Duration::from_secs(60)), {
                    let first_failed_txn = &txns[failure.transaction_index];
                    let sender = first_failed_txn.sender();
                    let payload = first_failed_txn.payload().payload_type();

                    let first_failed_txn_info = format!(
                        "due to {:?}, for account {}, max gas {}, payload {}",
                        failure,
                        first_failed_txn.sender(),
                        first_failed_txn.max_gas_amount(),
                        payload,
                    );

                    // TODO[Orderless]: Update this code, as transactions could be orderless and the account could be stateless.
                    let last_transactions =
                        if let Ok(account) = client.get_account_bcs(sender).await {
                            client
                                .get_account_ordered_transactions_bcs(
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
                        .view_apt_account_balance(sender)
                        .await
                        .map_or(-1, |v| v.into_inner() as i64);

                    warn!(
                        "[{:?}] Failed to submit {} txns in a batch, first failure: {}, chain id: {:?}, first asked: {}, failed seq nums: {:?}, failed error codes: {:?}, balance of {} and last transaction for account: {:?}",
                        client.path_prefix_string(),
                        failures.len(),
                        first_failed_txn_info,
                        txns[0].chain_id(),
                        txns[0].sequence_number(),
                        failures.iter().map(|f| txns[f.transaction_index].sequence_number()).collect::<Vec<_>>(),
                        by_error,
                        balance,
                        last_transactions,
                    );
                });
            }
        },
    };
}
