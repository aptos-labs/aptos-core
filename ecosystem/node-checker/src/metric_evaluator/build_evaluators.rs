// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::EvaluatorArgs,
    metric_evaluator::{
        ConsensusProposalsEvaluator, MetricsEvaluator, StateSyncMetricsEvaluator,
        CONSENSUS_PROPOSALS_EVALUATOR_NAME, STATE_SYNC_EVALUATOR_NAME,
    },
};
use anyhow::Result;
use std::collections::HashSet;

pub fn build_evaluators(
    evaluators_strings: &mut HashSet<String>,
    evaluator_args: &EvaluatorArgs,
) -> Result<Vec<Box<dyn MetricsEvaluator>>> {
    let mut evaluators: Vec<Box<dyn MetricsEvaluator>> = vec![];

    if evaluators_strings.take(STATE_SYNC_EVALUATOR_NAME).is_some() {
        evaluators.push(Box::new(StateSyncMetricsEvaluator::new(
            evaluator_args.state_sync_evaluator_args.clone(),
        )));
    }

    if evaluators_strings
        .take(CONSENSUS_PROPOSALS_EVALUATOR_NAME)
        .is_some()
    {
        evaluators.push(Box::new(ConsensusProposalsEvaluator::new(
            evaluator_args
                .consensus_evaluator_args
                .proposals_evaluator_args
                .clone(),
        )));
    }

    Ok(evaluators)
}
