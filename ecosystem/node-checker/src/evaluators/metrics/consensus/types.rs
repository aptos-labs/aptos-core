// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

pub const CATEGORY: &str = "consensus";

pub use super::proposals::{ConsensusProposalsEvaluator, ConsensusProposalsEvaluatorArgs};

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct ConsensusEvaluatorArgs {
    #[clap(flatten)]
    pub consensus_proposals_args: ConsensusProposalsEvaluatorArgs,
}
