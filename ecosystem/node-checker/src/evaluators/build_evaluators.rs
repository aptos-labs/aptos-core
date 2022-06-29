// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::EvaluatorArgs,
    evaluator::Evaluator,
    evaluators::{
        metrics::{
            ConsensusProposalsEvaluator, MetricsEvaluatorError, MetricsEvaluatorInput,
            StateSyncVersionEvaluator,
        },
        system_information::{
            SystemInformationBuildVersionEvaluator, SystemInformationEvaluatorError,
            SystemInformationEvaluatorInput,
        },
    },
};

use log::info;

use anyhow::{bail, Result};
use std::collections::HashSet;

/// This type is essential to making it possible to represent all
/// evaluators using a single trait, Evaluator. That trait has two
/// associated types, Input and Error. In order to build all evaluators
/// in one place, store them in a single vec, and then call all of them
/// in a single loop, we need this enum to represent how to call the
/// different Evaluator variants (based on Input + Error). In order to
/// use any collection of different implementations of a trait, we need
/// to use dynamic dispatch. This means the trait needs to be object-safe,
/// which places certain constraints on the trait. For more on this topic,
/// see https://doc.rust-lang.org/reference/items/traits.html#object-safety.
#[derive(Debug)]
pub enum EvaluatorType {
    Metrics(Box<dyn Evaluator<Input = MetricsEvaluatorInput, Error = MetricsEvaluatorError>>),
    SystemInformation(
        Box<
            dyn Evaluator<
                Input = SystemInformationEvaluatorInput,
                Error = SystemInformationEvaluatorError,
            >,
        >,
    ),
}

pub fn build_evaluators(
    evaluator_names: &[String],
    evaluator_args: &EvaluatorArgs,
) -> Result<Vec<EvaluatorType>> {
    let mut evaluator_names: HashSet<String> = evaluator_names.iter().cloned().collect();
    let mut evaluators: Vec<EvaluatorType> = vec![];

    let name = ConsensusProposalsEvaluator::get_name();
    match evaluator_names.take(&name) {
        Some(_) => {
            evaluators.push(EvaluatorType::Metrics(Box::new(
                ConsensusProposalsEvaluator::from_evaluator_args(evaluator_args),
            )));
        }
        None => log_did_not_build(&name),
    }

    let name = StateSyncVersionEvaluator::get_name();
    match evaluator_names.take(&name) {
        Some(_) => {
            evaluators.push(EvaluatorType::Metrics(Box::new(
                StateSyncVersionEvaluator::from_evaluator_args(evaluator_args),
            )));
        }
        None => log_did_not_build(&name),
    }

    let name = SystemInformationBuildVersionEvaluator::get_name();
    match evaluator_names.take(&name) {
        Some(_) => {
            evaluators.push(EvaluatorType::SystemInformation(Box::new(
                SystemInformationBuildVersionEvaluator::from_evaluator_args(evaluator_args),
            )));
        }
        None => log_did_not_build(&name),
    }

    if !evaluator_names.is_empty() {
        bail!(
            "The given evaluator names were unexpected: {:?}",
            evaluator_names
        );
    }

    Ok(evaluators)
}

fn log_did_not_build(name: &str) {
    info!("Did not build evaluator {}", name);
}
