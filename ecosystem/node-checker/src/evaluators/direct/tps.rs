// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
};
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Duration;
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

    /// We return this error if the transaction emitter failed to emit
    /// more transactions than the configured min TPS. This implies
    /// a configuration error.
    #[error("The transaction emitter only submitted {0} TPS but the minimum TPS requirement is {1}, this implies a configuration problem with the NHC instance")]
    InsufficientSubmittedTransactionsError(u64, u64),
}

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

    /// The minimum TPS required to pass the test.
    #[clap(long, default_value_t = 1000)]
    pub minimum_tps: u64,

    /// The number of times to repeat the target. This influences thread
    /// count and rest client count.
    #[clap(long, default_value_t = 1)]
    pub repeat_target_count: usize,
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
}

#[async_trait::async_trait]
impl Evaluator for TpsEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = TpsEvaluatorError;

    // You'll see that we're using the baseline chain ID here. This is okay
    // because at this point we've already asserted the baseline and target
    // have the same chain id.

    /// This test runs a TPS (transactions per second) evaluation on the target
    /// node, in which it passes if it meets some preconfigured minimum.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let target_url = input.target_node_address.get_api_url();

        let cluster_args = ClusterArgs {
            targets: vec![target_url; self.args.repeat_target_count],
            reuse_accounts: false,
            mint_args: self.args.mint_args.clone(),
            chain_id: input.baseline_node_information.chain_id,
        };
        let cluster = Cluster::try_from_cluster_args(&cluster_args)
            .await
            .map_err(TpsEvaluatorError::BuildClusterError)?;

        let stats = emit_transactions_with_cluster(&cluster, &self.args.emit_args, false)
            .await
            .map_err(TpsEvaluatorError::TransactionEmitterError)?;

        // AKA stats per second.
        let rate = stats.rate(Duration::from_secs(self.args.emit_args.duration));

        if rate.submitted < self.args.minimum_tps {
            return Err(TpsEvaluatorError::InsufficientSubmittedTransactionsError(
                rate.submitted,
                self.args.minimum_tps,
            ));
        }

        let mut description = format!("The minimum TPS (transactions per second) \
            required of nodes is {}, your node hit: {} (out of {} transactions submitted per second).", self.args.minimum_tps, rate.committed, rate.submitted);
        let evaluation_result = if rate.committed >= self.args.minimum_tps {
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
            )
        } else {
            description.push_str(
                " This implies that the hardware you're \
            using to run your node isn't powerful enough, please see the attached link",
            );
            self.build_evaluation_result_with_links(
                "Transaction processing speed is too low".to_string(),
                0,
                description,
                vec![NODE_REQUIREMENTS_LINK.to_string()],
            )
        };

        Ok(vec![evaluation_result])
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "tps".to_string()
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

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Tps(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}
