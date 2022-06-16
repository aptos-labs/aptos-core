// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use crate::{
    configuration::EvaluatorArgs, system_information_evaluator::SystemInformationEvaluator,
};
use anyhow::{bail, Result};
use log::info;
use std::collections::HashSet;

pub fn build_evaluators(
    evaluators: &[String],
    evaluator_args: &EvaluatorArgs,
) -> Result<Vec<Box<dyn SystemInformationEvaluator>>> {
    let evaluator_strings: HashSet<String> = evaluators.iter().cloned().collect();
    if evaluator_strings.is_empty() {
        bail!("No evaluators specified");
    }

    let mut evaluators: Vec<Box<dyn SystemInformationEvaluator>> = vec![];

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
