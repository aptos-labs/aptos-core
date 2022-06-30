// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use crate::{
    configuration::EvaluatorArgs,
    system_information_evaluator::{
        build_version_evaluator::BUILD_VERSION_EVALUATOR_NAME, BuildVersionEvaluator,
        SystemInformationEvaluator,
    },
};
use anyhow::Result;
use std::collections::HashSet;

pub fn build_evaluators(
    evaluators_strings: &mut HashSet<String>,
    evaluator_args: &EvaluatorArgs,
) -> Result<Vec<Box<dyn SystemInformationEvaluator>>> {
    let mut evaluators: Vec<Box<dyn SystemInformationEvaluator>> = vec![];

    if evaluators_strings
        .take(BUILD_VERSION_EVALUATOR_NAME)
        .is_some()
    {
        evaluators.push(Box::new(BuildVersionEvaluator::new(
            evaluator_args.build_version_evaluator_args.clone(),
        )));
    }

    Ok(evaluators)
}
