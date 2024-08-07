// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use crate::payload_client::user;
#[cfg(test)]
use crate::payload_client::validator::DummyValidatorTxnClient;
use crate::{
    error::QuorumStoreError,
    payload_client::{user::UserPayloadClient, PayloadClient},
};
use aptos_consensus_types::common::{Payload, PayloadFilter};
use aptos_logger::debug;
use aptos_types::{on_chain_config::ValidatorTxnConfig, validator_txn::ValidatorTransaction};
use aptos_validator_transaction_pool as vtxn_pool;
use fail::fail_point;
use futures::future::BoxFuture;
#[cfg(test)]
use std::collections::HashSet;
use std::{
    cmp::min,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct MixedPayloadClient {
    validator_txn_config: ValidatorTxnConfig,
    validator_txn_pool_client: Arc<dyn crate::payload_client::validator::ValidatorTxnPayloadClient>,
    user_payload_client: Arc<dyn UserPayloadClient>,
}

impl MixedPayloadClient {
    pub fn new(
        validator_txn_config: ValidatorTxnConfig,
        validator_txn_pool_client: Arc<
            dyn crate::payload_client::validator::ValidatorTxnPayloadClient,
        >,
        user_payload_client: Arc<dyn UserPayloadClient>,
    ) -> Self {
        Self {
            validator_txn_config,
            validator_txn_pool_client,
            user_payload_client,
        }
    }

    /// When enabled in smoke tests, generate 2 random validator transactions, 1 valid, 1 invalid.
    fn extra_test_only_vtxns(&self) -> Vec<ValidatorTransaction> {
        fail_point!("mixed_payload_client::extra_test_only_vtxns", |_| {
            use aptos_types::dkg::{DKGTranscript, DKGTranscriptMetadata};
            use move_core_types::account_address::AccountAddress;

            vec![ValidatorTransaction::DKGResult(DKGTranscript {
                metadata: DKGTranscriptMetadata {
                    epoch: 999,
                    author: AccountAddress::ZERO,
                },
                transcript_bytes: vec![],
            })]
        });
        vec![]
    }
}

#[async_trait::async_trait]
impl PayloadClient for MixedPayloadClient {
    async fn pull_payload(
        &self,
        mut max_poll_time: Duration,
        mut max_items: u64,
        mut max_items_after_filtering: u64,
        mut soft_max_items_after_filtering: u64,
        mut max_bytes: u64,
        max_inline_items: u64,
        max_inline_bytes: u64,
        validator_txn_filter: vtxn_pool::TransactionFilter,
        user_txn_filter: PayloadFilter,
        wait_callback: BoxFuture<'static, ()>,
        pending_ordering: bool,
        pending_uncommitted_blocks: usize,
        recent_max_fill_fraction: f32,
        block_timestamp: Duration,
    ) -> anyhow::Result<(Vec<ValidatorTransaction>, Payload), QuorumStoreError> {
        // Pull validator txns first.
        let validator_txn_pull_timer = Instant::now();
        let mut validator_txns = self
            .validator_txn_pool_client
            .pull(
                max_poll_time,
                min(
                    max_items,
                    self.validator_txn_config.per_block_limit_txn_count(),
                ),
                min(
                    max_bytes,
                    self.validator_txn_config.per_block_limit_total_bytes(),
                ),
                validator_txn_filter,
            )
            .await;

        validator_txns.extend(self.extra_test_only_vtxns());

        debug!("num_validator_txns={}", validator_txns.len());
        // Update constraints with validator txn pull results.
        max_items -= validator_txns.len() as u64;
        max_items_after_filtering -= validator_txns.len() as u64;
        soft_max_items_after_filtering -= validator_txns.len() as u64;
        max_bytes -= validator_txns
            .iter()
            .map(|txn| txn.size_in_bytes())
            .sum::<usize>() as u64;
        max_poll_time = max_poll_time.saturating_sub(validator_txn_pull_timer.elapsed());

        // Pull user payload.
        let user_payload = self
            .user_payload_client
            .pull(
                max_poll_time,
                max_items,
                max_items_after_filtering,
                soft_max_items_after_filtering,
                max_bytes,
                max_inline_items,
                max_inline_bytes,
                user_txn_filter,
                wait_callback,
                pending_ordering,
                pending_uncommitted_blocks,
                recent_max_fill_fraction,
                block_timestamp,
            )
            .await?;

        Ok((validator_txns, user_payload))
    }
}

#[tokio::test]
async fn mixed_payload_client_should_prioritize_validator_txns() {
    let all_validator_txns = vec![
        ValidatorTransaction::dummy(b"1".to_vec()),
        ValidatorTransaction::dummy(b"22".to_vec()),
        ValidatorTransaction::dummy(b"333".to_vec()),
    ];

    let all_user_txns = crate::test_utils::create_vec_signed_transactions(10);
    let client = MixedPayloadClient {
        validator_txn_config: ValidatorTxnConfig::V1 {
            per_block_limit_txn_count: 99,
            per_block_limit_total_bytes: 1048576,
        },
        validator_txn_pool_client: Arc::new(DummyValidatorTxnClient::new(
            all_validator_txns.clone(),
        )),
        user_payload_client: Arc::new(user::DummyClient::new(all_user_txns.clone())),
    };

    let (pulled_validator_txns, Payload::DirectMempool(pulled_user_txns)) = client
        .pull_payload(
            Duration::from_secs(1), // max_poll_time
            120,                    // max_items
            99,                     // max_unique_items
            99,                     // soft max_unique_items
            1048576,                // size limit: 1MB
            50,
            500000, // inline limit: 500KB
            vtxn_pool::TransactionFilter::PendingTxnHashSet(HashSet::new()),
            PayloadFilter::Empty,
            Box::pin(async {}),
            false,
            0,
            0.,
            aptos_infallible::duration_since_epoch(),
        )
        .await
        .unwrap()
    else {
        unreachable!()
    };

    assert_eq!(3, pulled_validator_txns.len());
    assert_eq!(10, pulled_user_txns.len());

    let (pulled_validator_txns, Payload::DirectMempool(pulled_user_txns)) = client
        .pull_payload(
            Duration::from_micros(500), // max_poll_time
            120,                        // max_items
            99,                         // max_unique_items
            99,                         // soft max_unique_items
            1048576,                    // size limit: 1MB
            50,
            500000, // inline limit: 500KB
            vtxn_pool::TransactionFilter::PendingTxnHashSet(HashSet::new()),
            PayloadFilter::Empty,
            Box::pin(async {}),
            false,
            0,
            0.,
            aptos_infallible::duration_since_epoch(),
        )
        .await
        .unwrap()
    else {
        unreachable!()
    };

    assert_eq!(1, pulled_validator_txns.len());
    assert_eq!(0, pulled_user_txns.len());

    let (pulled_validator_txns, Payload::DirectMempool(pulled_user_txns)) = client
        .pull_payload(
            Duration::from_secs(1), // max_poll_time
            2,                      // max_items
            2,                      // max_unique_items
            2,                      // soft max_unique_items
            1048576,                // size limit: 1MB
            0,
            0, // inline limit: 0
            vtxn_pool::TransactionFilter::PendingTxnHashSet(HashSet::new()),
            PayloadFilter::Empty,
            Box::pin(async {}),
            false,
            0,
            0.,
            aptos_infallible::duration_since_epoch(),
        )
        .await
        .unwrap()
    else {
        unreachable!()
    };

    assert_eq!(2, pulled_validator_txns.len());
    assert_eq!(0, pulled_user_txns.len());

    let (pulled_validator_txns, Payload::DirectMempool(pulled_user_txns)) = client
        .pull_payload(
            Duration::from_secs(1), // max_poll_time
            120,                    // max_items
            99,                     // max_unique_items
            99,                     // soft max_unique_items
            all_validator_txns[0].size_in_bytes() as u64,
            50,
            all_validator_txns[0].size_in_bytes() as u64,
            vtxn_pool::TransactionFilter::PendingTxnHashSet(HashSet::new()),
            PayloadFilter::Empty,
            Box::pin(async {}),
            false,
            0,
            0.,
            aptos_infallible::duration_since_epoch(),
        )
        .await
        .unwrap()
    else {
        unreachable!()
    };

    assert_eq!(1, pulled_validator_txns.len());
    assert_eq!(0, pulled_user_txns.len());
}

#[tokio::test]
async fn mixed_payload_client_should_respect_validator_txn_feature_flag() {
    let all_validator_txns = vec![
        ValidatorTransaction::dummy(b"1".to_vec()),
        ValidatorTransaction::dummy(b"22".to_vec()),
        ValidatorTransaction::dummy(b"333".to_vec()),
    ];

    let all_user_txns = crate::test_utils::create_vec_signed_transactions(10);
    let client = MixedPayloadClient {
        validator_txn_config: ValidatorTxnConfig::default_disabled(),
        validator_txn_pool_client: Arc::new(DummyValidatorTxnClient::new(
            all_validator_txns.clone(),
        )),
        user_payload_client: Arc::new(user::DummyClient::new(all_user_txns.clone())),
    };

    let (pulled_validator_txns, Payload::DirectMempool(pulled_user_txns)) = client
        .pull_payload(
            Duration::from_millis(50), // max_poll_time
            120,                       // max_items
            99,                        // max_unique_items
            99,                        // soft max_unique_items
            1048576,                   // size limit: 1MB
            50,
            500000, // inline limit: 500KB
            vtxn_pool::TransactionFilter::PendingTxnHashSet(HashSet::new()),
            PayloadFilter::Empty,
            Box::pin(async {}),
            false,
            0,
            0.,
            aptos_infallible::duration_since_epoch(),
        )
        .await
        .unwrap()
    else {
        unreachable!()
    };

    assert_eq!(0, pulled_validator_txns.len());
    assert_eq!(10, pulled_user_txns.len());
}
