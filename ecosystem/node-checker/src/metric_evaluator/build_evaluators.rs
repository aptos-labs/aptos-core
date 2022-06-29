// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::EvaluatorArgs,
    metric_evaluator::{
        ConsensusProposalsEvaluator, MetricsEvaluator, StateSyncMetricsEvaluator,
        CONSENSUS_PROPOSALS_EVALUATOR_NAME, STATE_SYNC_EVALUATOR_NAME,
    },
};
use anyhow::{bail, Result};
use log::info;
use std::collections::HashSet;

pub fn build_evaluators(
    evaluators: &[String],
    evaluator_args: &EvaluatorArgs,
) -> Result<Vec<Box<dyn MetricsEvaluator>>> {
    let evaluator_strings: HashSet<String> = evaluators.iter().cloned().collect();
    if evaluator_strings.is_empty() {
        bail!("No evaluators specified");
    }

    let mut evaluators: Vec<Box<dyn MetricsEvaluator>> = vec![];

    if evaluator_strings.contains(STATE_SYNC_EVALUATOR_NAME) {
        evaluators.push(Box::new(StateSyncMetricsEvaluator::new(
            evaluator_args.state_sync_evaluator_args.clone(),
        )));
    }

    if evaluator_strings.contains(CONSENSUS_PROPOSALS_EVALUATOR_NAME) {
        evaluators.push(Box::new(ConsensusProposalsEvaluator::new(
            evaluator_args
                .consensus_evaluator_args
                .proposals_evaluator_args
                .clone(),
        )));
    }

    let in_use_evaluators_names = evaluators
        .iter()
        .map(|e| e.get_name())
        .collect::<HashSet<_>>();
    for evaluator_string in evaluator_strings {
        if !in_use_evaluators_names.contains(&evaluator_string) {
            bail!("Evaluator {} does not exist", evaluator_string);
        }
    }

    info!(
        "Running with the following evaluators: {:?}",
        in_use_evaluators_names
    );

    Ok(evaluators)
}
