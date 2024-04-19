// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{adapter::TLedgerInfoProvider, dag_store::DagStore},
    error::QuorumStoreError,
    payload_client::PayloadClient,
};
use aptos_consensus_types::common::{Payload, PayloadFilter};
use aptos_logger::debug;
use aptos_types::validator_txn::ValidatorTransaction;
use aptos_validator_transaction_pool::TransactionFilter;
use arc_swap::ArcSwapOption;
use futures::future::BoxFuture;
use std::{ops::Deref, sync::Arc, time::Duration};
// use crate::state_replication::PayloadClient;

pub(crate) struct ShoalppPayloadClient {
    dag_store_vec: Vec<Arc<ArcSwapOption<DagStore>>>,
    payload_client: Arc<dyn PayloadClient>,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    window_size_config: u64,
}

impl ShoalppPayloadClient {
    pub fn new(
        dag_store_vec: Vec<Arc<ArcSwapOption<DagStore>>>,
        payload_client: Arc<dyn PayloadClient>,
        ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
        window_size_config: u64,
    ) -> Self {
        Self {
            dag_store_vec,
            payload_client,
            ledger_info_provider,
            window_size_config,
        }
    }

    fn get_payload_filter(&self, dag_vec: Vec<Arc<DagStore>>) -> PayloadFilter {
        let mut dag_reader_vec = Vec::new();
        dag_vec.iter().for_each(|dag| {
            dag_reader_vec.push(dag.read());
        });

        let mut exclude_payloads = Vec::new();
        dag_reader_vec
            .iter()
            .enumerate()
            .for_each(|(dag_id, dag_reader)| {
                let highest_round_nodes = dag_reader.highest_round_nodes();
                // TODO: support this for three dags
                let highest_commit_round = self
                    .ledger_info_provider
                    .get_highest_committed_anchor_round(dag_id as u8);
                let exclude_payload = if highest_round_nodes.is_empty() {
                    Vec::new()
                } else {
                    dag_reader
                        .reachable(
                            highest_round_nodes.iter().map(|node| node.metadata()),
                            Some(highest_commit_round.saturating_sub(self.window_size_config)),
                            |_| true,
                        )
                        .map(|node_status| node_status.as_node().payload())
                        .collect()
                };
                exclude_payloads.extend(exclude_payload);
            });
        PayloadFilter::from(&exclude_payloads)
    }
}

#[async_trait::async_trait]
impl PayloadClient for ShoalppPayloadClient {
    async fn pull_payload(
        &self,
        max_poll_time: Duration,
        max_items: u64,
        max_bytes: u64,
        max_inline_items: u64,
        max_inline_bytes: u64,
        validator_txn_filter: TransactionFilter,
        user_txn_filter: PayloadFilter,
        wait_callback: BoxFuture<'static, ()>,
        pending_ordering: bool,
        pending_uncommitted_blocks: usize,
        recent_max_fill_fraction: f32,
    ) -> anyhow::Result<(Vec<ValidatorTransaction>, Payload), QuorumStoreError> {
        let dag_vec = self
            .dag_store_vec
            .iter()
            .filter_map(|aso_dag| aso_dag.load().deref().clone())
            .collect();

        let payload_filter = self.get_payload_filter(dag_vec);

        debug!("[Bolt] pulling payload");
        self.payload_client
            .pull_payload(
                Duration::from_millis(100),
                max_items,
                max_bytes,
                max_inline_items,
                max_inline_bytes,
                validator_txn_filter,
                user_txn_filter,
                wait_callback,
                pending_ordering,
                pending_uncommitted_blocks,
                recent_max_fill_fraction,
            )
            .await
    }
}
