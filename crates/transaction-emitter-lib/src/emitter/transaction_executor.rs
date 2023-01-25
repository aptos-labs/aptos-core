// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::RETRY_POLICY;
use crate::transaction_generator::TransactionExecutor;
use anyhow::{format_err, Result};
use aptos_logger::{sample, sample::SampleRate, warn};
use aptos_rest_client::{error::RestError, Client as RestClient};
use aptos_sdk::{
    move_types::account_address::AccountAddress, types::transaction::SignedTransaction,
};
use async_trait::async_trait;
use futures::future::join_all;
use rand::{seq::SliceRandom, thread_rng};
use std::{sync::atomic::AtomicUsize, time::Duration};

// Reliable/retrying transaction executor, used for initializing
pub struct RestApiTransactionExecutor {
    pub rest_clients: Vec<RestClient>,
}

impl RestApiTransactionExecutor {
    fn random_rest_client(&self) -> &RestClient {
        let mut rng = thread_rng();
        self.rest_clients.choose(&mut rng).unwrap()
    }

    async fn submit_and_check(
        &self,
        txn: &SignedTransaction,
        failure_counter: &AtomicUsize,
    ) -> Result<()> {
        let rest_client = self.random_rest_client();
        if let Err(err) = rest_client.submit_bcs(txn).await {
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                warn!(
                    "[{}] Failed submitting transaction: {}",
                    rest_client.path_prefix_string(),
                    err,
                )
            );
            failure_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // even if txn fails submitting, it might get committed, so wait to see if that is the case.
        }
        if let Err(err) = rest_client
            .wait_for_transaction_by_hash(
                txn.clone().committed_hash(),
                txn.expiration_timestamp_secs(),
                None,
                Some(Duration::from_secs(10)),
            )
            .await
        {
            sample!(
                SampleRate::Duration(Duration::from_secs(60)),
                warn!(
                    "[{}] Failed waiting on a transaction: {}",
                    rest_client.path_prefix_string(),
                    err,
                )
            );
            failure_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Err(err)?;
        }
        Ok(())
    }
}

#[async_trait]
impl TransactionExecutor for RestApiTransactionExecutor {
    async fn get_account_balance(&self, account_address: AccountAddress) -> Result<u64> {
        Ok(RETRY_POLICY
            .retry(move || {
                self.random_rest_client()
                    .get_account_balance(account_address)
            })
            .await?
            .into_inner()
            .get())
    }

    async fn query_sequence_number(&self, account_address: AccountAddress) -> Result<u64> {
        Ok(RETRY_POLICY
            .retry(move || self.random_rest_client().get_account_bcs(account_address))
            .await?
            .into_inner()
            .sequence_number())
    }

    async fn execute_transactions(&self, txns: &[SignedTransaction]) -> Result<()> {
        self.execute_transactions_with_counter(txns, &AtomicUsize::new(0))
            .await
    }

    async fn execute_transactions_with_counter(
        &self,
        txns: &[SignedTransaction],
        failure_counter: &AtomicUsize,
    ) -> Result<()> {
        join_all(txns.iter().map(|txn| async move {
            let submit_result = RETRY_POLICY
                .retry(move || self.submit_and_check(txn, failure_counter))
                .await;
            if let Err(e) = submit_result {
                warn!("Failed submitting transaction {:?} with {:?}", txn, e);
            }
        }))
        .await;

        // if submission timeouts, it might still get committed:
        join_all(txns.iter().map(|req| {
            self.random_rest_client()
                .wait_for_signed_transaction_bcs(req)
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, RestError>>()
        .map_err(|e| format_err!("Failed to commit transactions: {:?}", e))?;

        Ok(())
    }
}
