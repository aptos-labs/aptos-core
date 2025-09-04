// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckerData, CheckerTrait};
use crate::{
    common::{IpRangeManager, IpRangeManagerConfig},
    endpoints::{VelorTapError, RejectionReason, RejectionReasonCode},
};
use anyhow::Result;
use async_trait::async_trait;
use std::net::IpAddr;

pub struct IpBlocklistChecker {
    manager: IpRangeManager,
}

impl IpBlocklistChecker {
    pub fn new(config: IpRangeManagerConfig) -> Result<Self> {
        Ok(Self {
            manager: IpRangeManager::new(config)?,
        })
    }
}

#[async_trait]
impl CheckerTrait for IpBlocklistChecker {
    async fn check(
        &self,
        data: CheckerData,
        _dry_run: bool,
    ) -> Result<Vec<RejectionReason>, VelorTapError> {
        match &data.source_ip {
            IpAddr::V4(source_ip) => {
                if self.manager.ipv4_list.contains(source_ip) {
                    return Ok(vec![RejectionReason::new(
                        format!("IP {} is in blocklist", source_ip),
                        RejectionReasonCode::IpInBlocklist,
                    )]);
                }
            },
            IpAddr::V6(source_ip) => {
                if self.manager.ipv6_list.contains(source_ip) {
                    return Ok(vec![RejectionReason::new(
                        format!("IP {} is in blocklist", source_ip),
                        RejectionReasonCode::IpInBlocklist,
                    )]);
                }
            },
        }
        Ok(vec![])
    }

    fn cost(&self) -> u8 {
        1
    }
}
