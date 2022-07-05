// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::EvaluatorArgs,
    evaluator::Evaluator,
    evaluators::{
        direct::{
            ApiEvaluatorError, DirectEvaluatorInput, LatencyEvaluator, TpsEvaluator,
            TpsEvaluatorError, TransactionPresenceEvaluator,
        },
        metrics::{
            ConsensusProposalsEvaluator, MetricsEvaluatorError, MetricsEvaluatorInput,
            StateSyncVersionEvaluator,
        },
        system_information::{
            BuildVersionEvaluator, SystemInformationEvaluatorError, SystemInformationEvaluatorInput,
        },
    },
};
use anyhow::{bail, Result};
use std::collections::HashSet;

type ApiEvaluatorType = Box<dyn Evaluator<Input = DirectEvaluatorInput, Error = ApiEvaluatorError>>;
type MetricsEvaluatorType =
    Box<dyn Evaluator<Input = MetricsEvaluatorInput, Error = MetricsEvaluatorError>>;
type SystemInformationEvaluatorType = Box<
    dyn Evaluator<Input = SystemInformationEvaluatorInput, Error = SystemInformationEvaluatorError>,
>;
type TpsEvaluatorType = Box<dyn Evaluator<Input = DirectEvaluatorInput, Error = TpsEvaluatorError>>;

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
    Api(ApiEvaluatorType),
    Metrics(MetricsEvaluatorType),
    SystemInformation(SystemInformationEvaluatorType),
    Tps(TpsEvaluatorType),
}

#[derive(Debug)]
pub struct EvaluatorSet {
    pub evaluators: Vec<EvaluatorType>,
}

// TODO: Try to think of a smart way to just have `get_evaluators<T>` and it
// takes in an enum variant. I don't know if that's possible in Rust though,
// enum variants on their own are not really values and they're definitely
// not types.
impl EvaluatorSet {
    pub fn new(evaluators: Vec<EvaluatorType>) -> Self {
        Self { evaluators }
    }

    pub fn get_metrics_evaluators(&self) -> Vec<&MetricsEvaluatorType> {
        self.evaluators
            .iter()
            .filter_map(|evaluator| match evaluator {
                EvaluatorType::Metrics(evaluator) => Some(evaluator),
                _ => None,
            })
            .collect()
    }

    pub fn get_system_information_evaluators(&self) -> Vec<&SystemInformationEvaluatorType> {
        self.evaluators
            .iter()
            .filter_map(|evaluator| match evaluator {
                EvaluatorType::SystemInformation(evaluator) => Some(evaluator),
                _ => None,
            })
            .collect()
    }

    pub fn get_direct_evaluators(&self) -> Vec<&EvaluatorType> {
        self.evaluators
            .iter()
            .filter(|evaluator| matches!(evaluator, EvaluatorType::Api(_) | EvaluatorType::Tps(_)))
            .collect()
    }
}

pub fn build_evaluators(
    evaluator_identifiers: &[String],
    evaluator_args: &EvaluatorArgs,
) -> Result<EvaluatorSet> {
    let mut evaluator_identifiers: HashSet<String> =
        evaluator_identifiers.iter().cloned().collect();
    let mut evaluators: Vec<EvaluatorType> = vec![];

    ConsensusProposalsEvaluator::add_from_evaluator_args(
        &mut evaluators,
        &mut evaluator_identifiers,
        evaluator_args,
    )?;
    StateSyncVersionEvaluator::add_from_evaluator_args(
        &mut evaluators,
        &mut evaluator_identifiers,
        evaluator_args,
    )?;
    BuildVersionEvaluator::add_from_evaluator_args(
        &mut evaluators,
        &mut evaluator_identifiers,
        evaluator_args,
    )?;
    TpsEvaluator::add_from_evaluator_args(
        &mut evaluators,
        &mut evaluator_identifiers,
        evaluator_args,
    )?;
    LatencyEvaluator::add_from_evaluator_args(
        &mut evaluators,
        &mut evaluator_identifiers,
        evaluator_args,
    )?;
    TransactionPresenceEvaluator::add_from_evaluator_args(
        &mut evaluators,
        &mut evaluator_identifiers,
        evaluator_args,
    )?;

    if !evaluator_identifiers.is_empty() {
        bail!(
            "The given evaluator identifiers were unexpected: {:?}",
            evaluator_identifiers
        );
    }

    Ok(EvaluatorSet::new(evaluators))
}
