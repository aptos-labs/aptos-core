// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckerData, CheckerTrait};
use crate::endpoints::{VelorTapError, RejectionReason, RejectionReasonCode};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MagicHeaderCheckerConfig {
    pub magic_header_key: String,
    pub magic_header_value: String,
}

pub struct MagicHeaderChecker {
    config: MagicHeaderCheckerConfig,
}

impl MagicHeaderChecker {
    pub fn new(config: MagicHeaderCheckerConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl CheckerTrait for MagicHeaderChecker {
    async fn check(
        &self,
        data: CheckerData,
        _dry_run: bool,
    ) -> Result<Vec<RejectionReason>, VelorTapError> {
        let header_value = match data.headers.get(&self.config.magic_header_key) {
            Some(header_value) => header_value,
            None => {
                return Ok(vec![RejectionReason::new(
                    format!("Magic header {} not found", self.config.magic_header_key),
                    RejectionReasonCode::MagicHeaderIncorrect,
                )])
            },
        };
        if header_value != &self.config.magic_header_value {
            return Ok(vec![RejectionReason::new(
                format!(
                    "Magic header value wrong {} not found",
                    self.config.magic_header_key
                ),
                RejectionReasonCode::MagicHeaderIncorrect,
            )]);
        }
        Ok(vec![])
    }

    fn cost(&self) -> u8 {
        2
    }
}
