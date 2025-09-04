// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::BypasserTrait;
use crate::{
    checkers::CheckerData,
    common::{IpRangeManager, IpRangeManagerConfig},
};
use anyhow::Result;
use async_trait::async_trait;

pub struct IpAllowlistBypasser {
    manager: IpRangeManager,
}

impl IpAllowlistBypasser {
    pub fn new(config: IpRangeManagerConfig) -> Result<Self> {
        Ok(Self {
            manager: IpRangeManager::new(config)?,
        })
    }
}

#[async_trait]
impl BypasserTrait for IpAllowlistBypasser {
    async fn request_can_bypass(&self, data: CheckerData) -> Result<bool> {
        Ok(self.manager.contains_ip(&data.source_ip))
    }
}
