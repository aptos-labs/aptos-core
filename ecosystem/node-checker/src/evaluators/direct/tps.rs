// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
};
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, time::Duration};
use thiserror::Error as ThisError;
use transaction_emitter_lib::{
    emit_transactions_with_cluster, Cluster, ClusterArgs, EmitArgs, MintArgs,
};

use super::types::DirectEvaluatorInput;

const CATEGORY: &str = "performance";
const NODE_REQUIREMENTS_LINK: &str = "https://aptos.dev/nodes/ait/node-requirements";

#[derive(Debug, ThisError)]
pub enum TpsEvaluatorError {
    /// Failed to build the cluster for the transaction emitter. This
    /// represents an internal logic error.
    #[error("Error building the transaction emitter cluster: {0}")]
    BuildClusterError(anyhow::Error),

    /// There was an error from the transaction emitter that we suspect
    /// was our own fault, not the fault of the target node.
    #[error("Error from within the transaction emitter: {0}")]
    TransactionEmitterError(anyhow::Error),
}

// TODO: Improve the flags situation here. For example, we always want --burst
// to be set, there is no reason to accept this as an argument.

// As you can see, we skip most of the fields here in terms of generating
// the OpenAPI spec. The evaluator args structs are only used for informational
// purposes (e.g. via `/get_configurations`), which is best effort. It is
// infeasible to derive PoemObject throughout the codebase or manually implement
// the relevant trait, so we just skip the field.
#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct TpsEvaluatorArgs {
    #[clap(flatten)]
    pub emit_args: EmitArgs,

    // Ed25519PrivateKey, either on the CLI or from a file, for minting coins.
    // We choose to take this in in the baseline config because we can't
    // securely transmit this at request time over the wire.
    #[clap(flatten)]
    pub mint_args: MintArgs,

    /// If set, try to use public peers instead of localhost.
    #[clap(long)]
    pub vasp: bool,

    /// The minimum TPS required to pass the test.
    #[clap(long, default_value_t = 500)]
    pub minimum_tps: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct TpsEvaluator {
    args: TpsEvaluatorArgs,
}

impl TpsEvaluator {
    pub fn new(args: TpsEvaluatorArgs) -> Self {
        Self { args }
    }

    fn build_evaluation_result(
        &self,
        headline: String,
        score: u8,
        explanation: String,
        links: Vec<String>,
    ) -> EvaluationResult {
        EvaluationResult {
            headline,
            score,
            explanation,
            category: CATEGORY.to_string(),
            evaluator_name: Self::get_name(),
            links,
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for TpsEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = TpsEvaluatorError;

    /// todo
    /// explain how we don't check baseline here
    /// todo explain how using the baseline chain ID is okay bc at this point we've
    /// asserted the baseline and target have the same chain id.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let cluster_args = ClusterArgs {
            targets: vec![input.target_node_address.url.clone()],
            vasp: self.args.vasp,
            mint_args: self.args.mint_args.clone(),
            chain_id: input.baseline_node_information.chain_id,
        };
        let cluster = Cluster::try_from(&cluster_args)
            .context("Failed to build cluster")
            .map_err(TpsEvaluatorError::BuildClusterError)?;

        let stats =
            emit_transactions_with_cluster(&cluster, &self.args.emit_args, cluster_args.vasp)
                .await
                .map_err(TpsEvaluatorError::TransactionEmitterError)?;

        // AKA stats per second.
        let rate = stats.rate(Duration::from_secs(self.args.emit_args.duration));

        let mut description = format!("The minimum TPS (transactions per second) \
            required of nodes is {}, your node hit: {} (out of {} transactions submitted per second).", self.args.minimum_tps, rate.committed, rate.submitted);
        let evaluation_result = if rate.committed > self.args.minimum_tps {
            if stats.committed == stats.submitted {
                description.push_str(
                    " Your node could theoretically hit \
                even higher TPS, the evaluation suite only tests to check \
                your node meets the minimum requirements.",
                );
            }
            self.build_evaluation_result(
                "Transaction processing speed is sufficient".to_string(),
                100,
                description,
                vec![],
            )
        } else {
            description.push_str(
                " This implies that the hardware you're \
            using to run your node isn't powerful enough, please see the attached link",
            );
            self.build_evaluation_result(
                "Transaction processing speed is too low".to_string(),
                0,
                description,
                vec![NODE_REQUIREMENTS_LINK.to_string()],
            )
        };

        Ok(vec![evaluation_result])
    }

    fn get_name() -> String {
        format!("{}_tps", CATEGORY)
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        // Assert we can get the key. This can fail in a number of ways:
        //   - The file based option was chosen but the file wasn't there.
        //   - Either option was chosen but the content was invalid.
        evaluator_args
            .tps_args
            .mint_args
            .get_mint_key()
            .context("Failed to get private key for TPS evaluator")?;
        Ok(Self::new(evaluator_args.tps_args.clone()))
    }
}
