// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{configuration::NodeAddress, server::NodeInformation};
use aptos_rest_client::IndexResponse;
use aptos_sdk::types::chain_id::ChainId;

/// This type of evaluator just takes in the target node address and then
/// directly fetches whatever information it needs to perform the evaluation.
/// This is different to other evaluators, where the information they need is
/// fetched earlier and then passed in, so all they have to do is process it.
/// It also takes in the response from the API index, which we know we have in
/// all cases since it is necessary for the mandatory node identity evaluator.

#[derive(Debug)]
pub struct DirectEvaluatorInput {
    pub baseline_node_information: NodeInformation,
    pub target_node_address: NodeAddress,
    pub baseline_index_response: IndexResponse,
    pub target_index_response: IndexResponse,
}

impl DirectEvaluatorInput {
    pub fn get_baseline_chain_id(&self) -> ChainId {
        self.baseline_node_information.chain_id
    }

    pub fn get_target_chain_id(&self) -> ChainId {
        ChainId::new(self.target_index_response.chain_id)
    }
}
