// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{anchor_election::AnchorElection, dag_store::Dag, CertifiedNode},
    experimental::buffer_manager::OrderedBlocks,
};
use aptos_consensus_types::common::Round;
use aptos_crypto::HashValue;
use aptos_infallible::RwLock;
use aptos_types::{epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures};
use futures_channel::mpsc::UnboundedSender;
use std::sync::Arc;

pub struct CommitRule {
    epoch_state: Arc<EpochState>,
    committed_block_id: HashValue,
    committed_round: Round,
    dag: Arc<RwLock<Dag>>,
    anchor_election: Box<dyn AnchorElection>,
    commit_sender: UnboundedSender<OrderedBlocks>,
}

impl CommitRule {
    pub fn new(
        epoch_state: Arc<EpochState>,
        latest_ledger_info: LedgerInfoWithSignatures,
        dag: Arc<RwLock<Dag>>,
        anchor_election: Box<dyn AnchorElection>,
        commit_sender: UnboundedSender<OrderedBlocks>,
    ) -> Self {
        // TODO: we need to initialize the anchor election based on the dag
        Self {
            epoch_state,
            committed_block_id: latest_ledger_info.commit_info().id(),
            committed_round: latest_ledger_info.commit_info().round(),
            dag,
            anchor_election,
            commit_sender,
        }
    }

    pub fn new_node(&mut self, _node: &CertifiedNode) {
        todo!();
    }
}
