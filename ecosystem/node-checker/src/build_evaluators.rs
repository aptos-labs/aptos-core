// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    args::Args,
    metric_evaluator::{MetricsEvaluator, StateSyncMetricsEvaluator, STATE_SYNC_EVALUATOR_NAME},
};
use anyhow::{bail, Result};
use log::{info, warn};
use std::collections::HashSet;

pub fn build_evaluators(args: &Args) -> Result<Vec<Box<dyn MetricsEvaluator>>> {
    let evaluator_strings: HashSet<String> = args.evaluators.iter().cloned().collect();
    if evaluator_strings.is_empty() {
        bail!("No evaluators specified");
    }

    let mut evaluators: Vec<Box<dyn MetricsEvaluator>> = vec![];

    if evaluator_strings.contains(STATE_SYNC_EVALUATOR_NAME) {
        let state_sync_evaluator =
            StateSyncMetricsEvaluator::new(args.evaluator_args.state_sync_evaluator_args.clone());
        evaluators.push(Box::new(state_sync_evaluator));
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
