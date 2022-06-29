// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{configuration::NodeAddress, server::NodeInformation};

/// This type of evaluator just takes in the target node address and then
/// directly fetches whatever information it needs to perform the evaluation.
/// This is different to other evaluators, where the information they need is
/// fetched earlier and then passed in, so all they have to do is process it.

#[derive(Debug)]
pub struct DirectEvaluatorInput {
    pub baseline_node_information: NodeInformation,
    pub target_node_address: NodeAddress,
}
