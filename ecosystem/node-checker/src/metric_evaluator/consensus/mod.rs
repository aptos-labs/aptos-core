// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

pub const CONSENSUS_EVALUATOR_SOURCE: &str = "consensus";

mod proposals_evaluator;

pub use proposals_evaluator::{ConsensusProposalsEvaluator, CONSENSUS_PROPOSALS_EVALUATOR_NAME};

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct ConsensusMetricsEvaluatorArgs {
    #[clap(flatten)]
    pub proposals_evaluator_args: proposals_evaluator::ConsensusProposalsEvaluatorArgs,
}
