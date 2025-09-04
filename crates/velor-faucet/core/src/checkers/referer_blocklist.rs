// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckerData, CheckerTrait};
use crate::{
    common::{ListManager, ListManagerConfig},
    endpoints::{VelorTapError, RejectionReason, RejectionReasonCode},
};
use anyhow::Result;
use velor_logger::info;
use async_trait::async_trait;
use poem::http::header::REFERER;

pub struct RefererBlocklistChecker {
    manager: ListManager,
}

impl RefererBlocklistChecker {
    pub fn new(config: ListManagerConfig) -> Result<Self> {
        let manager = ListManager::new(config)?;
        info!(
            "Loaded {} items into RefererBlocklistChecker",
            manager.num_items()
        );
        Ok(Self { manager })
    }
}

#[async_trait]
impl CheckerTrait for RefererBlocklistChecker {
    async fn check(
        &self,
        data: CheckerData,
        _dry_run: bool,
    ) -> Result<Vec<RejectionReason>, VelorTapError> {
        let referer = match data.headers.get(REFERER).and_then(|v| v.to_str().ok()) {
            Some(referer) => referer,
            None => return Ok(vec![]),
        };
        if self.manager.contains(referer) {
            Ok(vec![RejectionReason::new(
                format!(
                    "The provided referer is not allowed by the server: {}",
                    referer
                ),
                RejectionReasonCode::RefererBlocklisted,
            )])
        } else {
            Ok(vec![])
        }
    }

    fn cost(&self) -> u8 {
        2
    }
}
