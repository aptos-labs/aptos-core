// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

pub const CATEGORY: &str = "state_sync";

pub use super::version::{StateSyncVersionEvaluator, StateSyncVersionEvaluatorArgs};

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct StateSyncEvaluatorArgs {
    #[clap(flatten)]
    pub state_sync_version_args: StateSyncVersionEvaluatorArgs,
}
