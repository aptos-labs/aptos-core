// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{
    common::{get_value, GetValueResult},
    types::{SystemInformationEvaluatorError, SystemInformationEvaluatorInput},
    CATEGORY,
};
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
    metric_collector::SystemInformation,
};
use anyhow::Result;
use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

// TODO: Use the keys in crates/aptos-telemetry/src/system_information.rs
const CPU_COUNT_KEY: &str = "cpu_count";
const MEMORY_TOTAL_KEY: &str = "memory_total";

const NODE_REQUIREMENTS_DOC_LINK: &str = "https://aptos.dev/nodes/ait/node-requirements";

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct HardwareEvaluatorArgs {
    /// The minimum number of physical CPU cores the machine must have.
    #[clap(long, default_value_t = 8)]
    pub min_cpu_cores: u64,

    /// The minimum amount of RAM in GB (not GiB) the machine must have.
    #[clap(long, default_value_t = 31)]
    pub min_ram_gb: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct HardwareEvaluator {
    args: HardwareEvaluatorArgs,
}

impl HardwareEvaluator {
    pub fn new(args: HardwareEvaluatorArgs) -> Self {
        Self { args }
    }

    // TODO: Make this more general, so we can use it in build_version too.
    fn get_system_information_value(
        &self,
        system_information: &SystemInformation,
        key: &str,
    ) -> GetValueResult {
        let evaluation_on_missing_fn = || {
            self.build_evaluation_result(
                format!("Key \"{}\" missing", key),
                0,
                format!(
                    "The key \"{}\" is missing from the system information reported by the node",
                    key
                ),
            )
        };
        get_value(system_information, key, evaluation_on_missing_fn)
    }

    fn evaluate_single_item(
        &self,
        input: &SystemInformationEvaluatorInput,
        key: &str,
        minimum: u64,
        unit: &str,
    ) -> EvaluationResult {
        let value_from_target =
            match self.get_system_information_value(&input.target_system_information, key) {
                GetValueResult::Present(value) => match value.parse::<u64>() {
                    Ok(value) => value,
                    Err(err) => {
                        return self.build_evaluation_result(
                            format!("Failed to parse value for key \"{}\" as an int", key),
                            0,
                            format!(
                                "The value ({}) for key \"{}\" could not be parsed as an int: {}",
                                value, key, err
                            ),
                        )
                    }
                },
                GetValueResult::Missing(evaluation_result) => {
                    return evaluation_result;
                }
            };

        if value_from_target < minimum {
            self.build_evaluation_result_with_links(
                format!("{} is too low", key),
                25,
                format!(
                    "The value for {} is too small: {} {}. It must be at least {} {}",
                    key, value_from_target, unit, minimum, unit
                ),
                vec![NODE_REQUIREMENTS_DOC_LINK.to_string()],
            )
        } else {
            self.build_evaluation_result_with_links(
                format!("{} is large enough", key),
                100,
                format!(
                    "The value for {} is large enough: {} {}. Great! The minimum is {} {}",
                    key, value_from_target, unit, minimum, unit
                ),
                vec![NODE_REQUIREMENTS_DOC_LINK.to_string()],
            )
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for HardwareEvaluator {
    type Input = SystemInformationEvaluatorInput;
    type Error = SystemInformationEvaluatorError;

    /// Assert that the build commit hashes match.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let evaluation_results = vec![
            self.evaluate_single_item(input, CPU_COUNT_KEY, self.args.min_cpu_cores, "cores"),
            self.evaluate_single_item(
                input,
                MEMORY_TOTAL_KEY,
                self.args.min_ram_gb * 1_000_000, // Convert from GB to KB
                "KB",
            ),
        ];

        Ok(evaluation_results)
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "hardware".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.hardware_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::SystemInformation(Box::new(
            Self::from_evaluator_args(evaluator_args)?,
        )))
    }
}
