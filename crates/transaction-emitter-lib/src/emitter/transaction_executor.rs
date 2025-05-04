// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::FETCH_ACCOUNT_RETRY_POLICY;
use anyhow::{Context, Result};
use aptos_logger::{sample, sample::SampleRate};
use aptos_rest_client::{aptos_api_types::AptosErrorCode, error::RestError, Client as RestClient};
use aptos_sdk::{
    move_types::account_address::AccountAddress, types::transaction::SignedTransaction,
};
use aptos_transaction_generator_lib::{CounterState, ReliableTransactionSubmitter};
use async_trait::async_trait;
use futures::future::join_all;
use log::{debug, info, warn};
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, Rng, SeedableRng};
use std::{
    sync::atomic::AtomicUsize,
    time::{Duration, Instant},
};

// Reliable/retrying transaction executor, used for initializing
pub struct RestApiReliableTransactionSubmitter {
    rest_clients: Vec<RestClient>,
    max_retries: usize,
    retry_after: Duration,
}

impl RestApiReliableTransactionSubmitter {
    pub fn new(rest_clients: Vec<RestClient>, max_retries: usize, retry_after: Duration) -> Self {
        info!(
            "Using reliable/retriable init transaction executor with {} retries, every {}s",
            max_retries,
            retry_after.as_secs_f32()
        );
        Self {
            rest_clients,
            max_retries,
            retry_after,
        }
    }

    fn random_rest_client(&self) -> &RestClient {
        let mut rng = thread_rng();
        self.rest_clients.choose(&mut rng).unwrap()
    }

    fn random_rest_client_from_rng<R>(&self, rng: &mut R) -> &RestClient
    where
        R: Rng + ?Sized,
    {
        self.rest_clients.choose(rng).unwrap()
    }

    async fn submit_check_and_retry(
        &self,
        txn: &SignedTransaction,
        counters: &CounterState,
        run_seed: u64,
    ) -> Result<()> {
        for i in 0..self.max_retries {
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                debug!(
                    "Running reliable/retriable fetching, current state: {}",
                    counters.show_detailed()
                )
            );

            // All transactions from the same sender, need to be submitted to the same client
            // in the same retry round, so that they are not placed in parking lot.
            // Do so by selecting a client via seeded random selection.
            let seed = [
                i.to_le_bytes().to_vec(),
                run_seed.to_le_bytes().to_vec(),
                txn.sender().to_vec(),
            ]
            .concat();
            let mut seeded_rng = StdRng::from_seed(*aptos_crypto::HashValue::sha3_256_of(&seed));
            let rest_client = self.random_rest_client_from_rng(&mut seeded_rng);
            let mut failed_submit = false;
            let mut failed_wait = false;
            let result = submit_and_check(
                rest_client,
                txn,
                self.retry_after,
                i == 0,
                &mut failed_submit,
                &mut failed_wait,
            )
            .await;

            if failed_submit {
                counters.submit_failures[i.min(counters.submit_failures.len() - 1)]
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if !counters.by_client.is_empty() {
                    counters
                        .by_client
                        .get(&rest_client.path_prefix_string())
                        .map(|(_, submit_failures, _)| {
                            submit_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                        });
                }
            }
            if failed_wait {
                counters.wait_failures[i.min(counters.wait_failures.len() - 1)]
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if !counters.by_client.is_empty() {
                    counters
                        .by_client
                        .get(&rest_client.path_prefix_string())
                        .map(|(_, _, wait_failures)| {
                            wait_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                        });
                }
            }

            match result {
                Ok(()) => {
                    counters
                        .successes
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if !counters.by_client.is_empty() {
                        counters
                            .by_client
                            .get(&rest_client.path_prefix_string())
                            .map(|(successes, _, _)| {
                                successes.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                            });
                    }
                    return Ok(());
                },
                Err(err) => {
                    // TODO: we should have a better way to decide if a failure is retryable
                    if format!("{}", err).contains("SEQUENCE_NUMBER_TOO_OLD") {
                        break;
                    }
                },
            }
        }

        // if submission timeouts, it might still get committed:
        let onchain_info = self
            .random_rest_client()
            .wait_for_signed_transaction_bcs(txn)
            .await?
            .into_inner()
            .info;
        if !onchain_info.status().is_success() {
            anyhow::bail!(
                "Transaction failed execution with {:?}",
                onchain_info.status()
            );
        }

        counters
            .successes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

async fn warn_detailed_error(
    call_name: &str,
    rest_client: &RestClient,
    txn: &SignedTransaction,
    err: Result<&aptos_types::transaction::TransactionInfo, &RestError>,
) {
    let sender = txn.sender();
    let payload = txn.payload().payload_type();
    let (last_transactions, seq_num) =
        if let Ok(account) = rest_client.get_account_bcs(sender).await {
            let inner = account.into_inner();
            (
                // TODO[Orderless]: Fetch previous sequence numbers doesn't make sense for orderless transactions.
                // What's the alternative?
                rest_client
                    .get_account_ordered_transactions_bcs(
                        sender,
                        Some(inner.sequence_number().saturating_sub(1)),
                        Some(5),
                    )
                    .await
                    .ok()
                    .map(|r| {
                        r.into_inner()
                            .into_iter()
                            .map(|t| t.info)
                            .collect::<Vec<_>>()
                    }),
                Some(inner.sequence_number()),
            )
        } else {
            (None, None)
        };
    let balance = rest_client
        .view_apt_account_balance(sender)
        .await
        .map_or(-1, |v| v.into_inner() as i128);

    warn!(
        "[{:?}] Failed {} transaction: {:?}, replay protector: {}, payload: {}, gas: unit {} and max {}, for account {}, last seq_num {:?}, balance of {} and last transaction for account: {:?}",
        rest_client.path_prefix_string(),
        call_name,
        err,
        txn.replay_protector(),
        payload,
        txn.gas_unit_price(),
        txn.max_gas_amount(),
        sender,
        seq_num,
        balance,
        last_transactions,
    );
}

async fn submit_and_check(
    rest_client: &RestClient,
    txn: &SignedTransaction,
    wait_duration: Duration,
    first_try: bool,
    failed_submit: &mut bool,
    failed_wait: &mut bool,
) -> Result<()> {
    let start = Instant::now();
    if let Err(err) = rest_client.submit_bcs(txn).await {
        sample!(
            SampleRate::Duration(Duration::from_secs(60)),
            warn_detailed_error("submitting", rest_client, txn, Err(&err)).await
        );
        *failed_submit = true;
        if first_try && format!("{}", err).contains("SEQUENCE_NUMBER_TOO_OLD") {
            sample!(
                SampleRate::Duration(Duration::from_secs(2)),
                warn_detailed_error("submitting on first try", rest_client, txn, Err(&err)).await
            );
            // There's no point to wait or retry on this error.
            // TODO: find a better way to propogate this error to the caller.
            Err(err)?
        } else {
            // even if txn fails submitting, it might get committed, so wait to see if that is the case.
        }
    }
    match rest_client
        .wait_for_transaction_by_hash_bcs(
            txn.committed_hash(),
            txn.expiration_timestamp_secs(),
            None,
            Some(wait_duration.saturating_sub(start.elapsed())),
        )
        .await
    {
        Err(err) => {
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                warn_detailed_error("waiting on a", rest_client, txn, Err(&err)).await
            );
            *failed_wait = true;
            Err(err)?;
        },
        Ok(result) => {
            let transaction_info = &result.inner().info;
            if !transaction_info.status().is_success() {
                sample!(
                    SampleRate::Duration(Duration::from_secs(60)),
                    warn_detailed_error("waiting on a", rest_client, txn, Ok(transaction_info))
                        .await
                );
                anyhow::bail!(
                    "Transaction failed execution with VM status {:?}",
                    transaction_info.status()
                );
            }
        },
    }

    Ok(())
}

pub async fn query_sequence_number_with_client(
    rest_client: &RestClient,
    account_address: AccountAddress,
) -> Result<u64> {
    let result = FETCH_ACCOUNT_RETRY_POLICY
        .retry_if(
            move || rest_client.get_account_sequence_number(account_address),
            |error: &RestError| !is_account_not_found(error),
        )
        .await;
    Ok(*result?.inner())
}

fn is_account_not_found(error: &RestError) -> bool {
    match error {
        RestError::Api(error) => matches!(error.error.error_code, AptosErrorCode::AccountNotFound),
        _ => false,
    }
}

#[async_trait]
impl ReliableTransactionSubmitter for RestApiReliableTransactionSubmitter {
    async fn get_account_balance(&self, account_address: AccountAddress) -> Result<u64> {
        Ok(FETCH_ACCOUNT_RETRY_POLICY
            .retry_if(
                move || {
                    self.random_rest_client()
                        .view_apt_account_balance(account_address)
                },
                |error: &RestError| match error {
                    RestError::Api(error) => !matches!(
                        error.error.error_code,
                        AptosErrorCode::AccountNotFound | AptosErrorCode::InvalidInput
                    ),
                    RestError::Unknown(_) => false,
                    _ => true,
                },
            )
            .await?
            .into_inner())
    }

    async fn query_sequence_number(&self, account_address: AccountAddress) -> Result<u64> {
        query_sequence_number_with_client(self.random_rest_client(), account_address).await
    }

    async fn execute_transactions_with_counter(
        &self,
        txns: &[SignedTransaction],
        counters: &CounterState,
    ) -> Result<()> {
        let run_seed: u64 = thread_rng().gen();

        join_all(
            txns.iter()
                .map(|txn| self.submit_check_and_retry(txn, counters, run_seed)),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<()>, anyhow::Error>>()
        .with_context(|| {
            format!(
                "Tried executing {} txns, request counters: {:?}",
                txns.len(),
                counters.show_detailed()
            )
        })?;

        Ok(())
    }

    fn create_counter_state(&self) -> CounterState {
        CounterState {
            submit_failures: std::iter::repeat_with(|| AtomicUsize::new(0))
                .take(self.max_retries)
                .collect(),
            wait_failures: std::iter::repeat_with(|| AtomicUsize::new(0))
                .take(self.max_retries)
                .collect(),
            successes: AtomicUsize::new(0),
            by_client: self
                .rest_clients
                .iter()
                .map(|client| {
                    (
                        client.path_prefix_string(),
                        (
                            AtomicUsize::new(0),
                            AtomicUsize::new(0),
                            AtomicUsize::new(0),
                        ),
                    )
                })
                .collect(),
        }
    }
}
