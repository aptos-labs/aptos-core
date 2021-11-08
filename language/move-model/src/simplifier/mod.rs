// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

use crate::model::GlobalEnv;

mod pass_inline;

/// Available simplifications passes to run after tbe model is built
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimplificationPass {
    Inline,
}

impl FromStr for SimplificationPass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = match s {
            "inline" => SimplificationPass::Inline,
            _ => return Err(s.to_string()),
        };
        Ok(r)
    }
}

impl fmt::Display for SimplificationPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inline => write!(f, "inline"),
        }
    }
}

impl SimplificationPass {
    pub fn run(&self, env: &mut GlobalEnv) -> Result<()> {
        match self {
            Self::Inline => pass_inline::run_pass_inline(env),
        }
    }
}
