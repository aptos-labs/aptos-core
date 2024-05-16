// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        adapter::TLedgerInfoProvider, dag_store::DagStore,
        observability::counters::PAYLOAD_FILTER_COUNT,
    },
    error::QuorumStoreError,
    monitor,
    payload_client::PayloadClient,
};
use aptos_consensus_types::common::{Author, Payload, PayloadFilter, Round, TransactionSummary};
use aptos_infallible::Mutex;
use aptos_logger::{debug, info};
use aptos_types::{epoch_state::EpochState, validator_txn::ValidatorTransaction};
use aptos_validator_transaction_pool::TransactionFilter;
use arc_swap::ArcSwapOption;
use futures::future::BoxFuture;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::{BTreeMap, VecDeque},
    iter,
    ops::{Add, Deref},
    sync::Arc,
    time::Duration,
};
// use crate::state_replication::PayloadClient;

pub(crate) struct ShoalppPayloadClient {
    dag_store_vec: Vec<Arc<ArcSwapOption<DagStore>>>,
    payload_client: Arc<dyn PayloadClient>,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    window_size_config: u64,
    exclude_state: Vec<Mutex<ExcludesState>>,
    self_author: Author,
}

#[derive(Default)]
struct ExcludesState {
    txn_len_by_round: BTreeMap<Round, usize>,
    txns: VecDeque<TransactionSummary>,
    last_highest: Round,
}

impl ShoalppPayloadClient {
    pub fn new(
        dag_store_vec: Vec<Arc<ArcSwapOption<DagStore>>>,
        payload_client: Arc<dyn PayloadClient>,
        ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
        window_size_config: u64,
        self_author: Author,
    ) -> Self {
        Self {
            dag_store_vec,
            payload_client,
            ledger_info_provider,
            window_size_config,
            exclude_state: vec![
                Mutex::new(ExcludesState::default()),
                Mutex::new(ExcludesState::default()),
                Mutex::new(ExcludesState::default()),
            ],
            self_author,
        }
    }

    fn get_payload_filter(&self, dag_vec: Vec<Arc<DagStore>>) -> PayloadFilter {
        let mut dag_reader_vec = Vec::new();
        dag_vec.iter().for_each(|dag| {
            dag_reader_vec.push(dag.read());
        });

        let mut excludes: Vec<_> = dag_reader_vec
            .par_iter()
            .enumerate()
            .map(|(dag_id, dag_reader)| {
                let current_highest_committed = self
                    .ledger_info_provider
                    .get_highest_committed_anchor_round(dag_id as u8);
                let mut state = self.exclude_state[dag_id].lock();

                if !state.txns.is_empty() {
                    let reminder = state
                        .txn_len_by_round
                        .split_off(&(current_highest_committed + 1));
                    let to_remove = state.txn_len_by_round.iter().map(|(_, count)| count).sum();
                    info!("exclude payload to_remove {}", to_remove);
                    info!("exclude payload remaining_rounds {}", reminder.len());
                    state.txn_len_by_round = reminder;
                    state.txns.drain(0..to_remove);
                }

                let highest_round_nodes = dag_reader.highest_round_nodes();
                let excludes = if !highest_round_nodes.is_empty() {
                    let target_round = highest_round_nodes.last().unwrap().metadata().round();
                    let upto_round = if state.txns.is_empty() {
                        current_highest_committed.saturating_sub(self.window_size_config)
                    } else {
                        target_round.min(state.last_highest)
                    };

                    (upto_round..=target_round)
                        .flat_map(|round| {
                            let Some(node) =
                                dag_reader.get_node_by_round_author(round, &self.self_author)
                            else {
                                return None;
                            };
                            let entry = state.txn_len_by_round.entry(node.round()).or_default();
                            let payload = node.payload();
                            *entry = *entry + payload.len();

                            if let Payload::DirectMempool(txns) = payload {
                                Some(txns)
                            } else {
                                None
                            }
                        })
                        .flatten()
                        .map(|txn| TransactionSummary {
                            sender: txn.sender(),
                            sequence_number: txn.sequence_number(),
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                info!("exclude payload len {}", excludes.len());

                for e in excludes {
                    state.txns.push_back(e);
                }

                state.last_highest = state.last_highest + 1;
                state.txns.clone()
            })
            .flatten()
            .collect();

        let last_proposals: Vec<_> = dag_reader_vec
            .par_iter()
            .flat_map(|dag_reader| &dag_reader.recent_proposal)
            .filter_map(|payload| {
                if let Payload::DirectMempool(txns) = payload {
                    Some(txns)
                } else {
                    None
                }
            })
            .flatten()
            .map(|txn| TransactionSummary {
                sender: txn.sender(),
                sequence_number: txn.sequence_number(),
            })
            .collect();

        excludes.extend_from_slice(&last_proposals);

        PAYLOAD_FILTER_COUNT.observe(excludes.len() as f64);

        PayloadFilter::DirectMempool(excludes)
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

        let user_txn_filter = monitor!(
            "dag_shoal_get_payload_filter",
            self.get_payload_filter(dag_vec)
        );

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
