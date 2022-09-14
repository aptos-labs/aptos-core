// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::{EvaluatorArgs, NodeAddress},
    evaluator::EvaluationResult,
    evaluators::EvaluatorType,
};
use log::info;
use std::{collections::HashSet, error::Error, fmt::Debug};

// You'll notice that a couple of the methods here require Self: Sized.
// The intent with the Evaluator trait is that a caller will build a vec
// of different evaluators with different associated input and error types.
// To do this, we need to construct trait objects (Box<dyn Evaluator<Input, Error>>),
// and to do that, the trait needs to be object safe. Object safe traits may
// not have functions or methods that do not take self as a parameter unless
// they are marked as `where Self: Sized`. This constraint means these functions
// can not be called on the trait object itself, just on the struct / Self.
// For more information on this topic, see:
// https://doc.rust-lang.org/reference/items/traits.html#object-safety

/// An Evaluator is a component of NHC that is responsible for evaluating
/// a particular aspect of the node under investigation, be that metrics,
/// system information, API checks, load tests, etc.
#[async_trait::async_trait]
pub trait Evaluator: Debug + Sync + Send {
    type Input: Debug;
    type Error: Error;

    /// This function is expected to take input, whatever that may be,
    /// and return a vec of evaluation results. It should only return
    /// errors when there is something wrong with NHC itself or its
    /// baseline configuration (e.g. a baseline node fails to return
    /// an expected value). If something is unexpected with the target,
    /// we expect this function to return an EvaluationResult indicating
    /// as such.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error>;

    /// All evaluators must have a category. This is used for building
    /// EvaluationResults.
    fn get_category_name() -> String
    where
        Self: Sized;

    /// All evaluators must have a name. We use this to select evaluators
    /// when building them from the initial configuration.
    fn get_evaluator_name() -> String
    where
        Self: Sized;

    /// This is the "fully qualified" identifier used for specifying that you
    /// want to use this evaluator.
    fn get_identifier() -> String
    where
        Self: Sized,
    {
        format!(
            "{}_{}",
            Self::get_category_name(),
            Self::get_evaluator_name()
        )
    }

    /// Before the evaluation is run, this function is called for all evaluators
    /// configured for the given baseline configuration. This gives those evaluators
    /// an opportunity to error out early if a necessary argument is not provided.
    fn validate_check_node_call(&self, _target_node_address: &NodeAddress) -> anyhow::Result<()> {
        Ok(())
    }

    // It would be better to require From<&EvaluatorArgs> on the trait
    // itself, but that has a few issues. First, it would introduce a
    // lifetime parameter on the trait, which makes it harder to use.
    // Second, I haven't found a way to add that supertrait in only the
    // case where Self: Sized, whereas I can do that with this function.

    /// We require this function to ensure we can build all evaluators
    /// from the top level evaluator args.
    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// This is useful for build_evaluators.
    fn evaluator_type_from_evaluator_args(
        evaluator_args: &EvaluatorArgs,
    ) -> anyhow::Result<EvaluatorType>
    where
        Self: Sized;

    /// This is useful for build_evaluators.
    /// There should be no reason to override the default impl.
    fn add_from_evaluator_args(
        evaluators: &mut Vec<EvaluatorType>,
        evaluator_identifiers: &mut HashSet<String>,
        evaluator_args: &EvaluatorArgs,
    ) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        let identifier = Self::get_identifier();
        match evaluator_identifiers.take(&identifier) {
            Some(_) => evaluators.push(Self::evaluator_type_from_evaluator_args(evaluator_args)?),
            None => info!("Did not build evaluator {}", identifier),
        };
        Ok(())
    }

    // Helper for building EvaluationResults with the name already filled in.
    fn build_evaluation_result(
        &self,
        headline: String,
        score: u8,
        explanation: String,
    ) -> EvaluationResult
    where
        Self: Sized,
    {
        self.build_evaluation_result_with_links(headline, score, explanation, vec![])
    }

    // Helper for building EvaluationResults with the name already filled in
    // and optionally with links.
    fn build_evaluation_result_with_links(
        &self,
        headline: String,
        score: u8,
        explanation: String,
        links: Vec<String>,
    ) -> EvaluationResult
    where
        Self: Sized,
    {
        EvaluationResult {
            headline,
            score,
            explanation,
            category: Self::get_category_name(),
            evaluator_name: Self::get_evaluator_name(),
            links,
        }
    }
}
