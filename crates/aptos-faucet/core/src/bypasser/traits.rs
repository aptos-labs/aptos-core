// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::checkers::CheckerData;
use anyhow::Result;
use async_trait::async_trait;

/// This trait defines something that checks whether a given request should
/// skip all the checkers and storage, for example an IP allowlist.
#[async_trait]
pub trait Bypasser: Sync + Send + 'static {
    /// Returns true if the request should be allowed to bypass all checkers
    /// and storage.
    async fn request_can_bypass(&self, data: CheckerData) -> Result<bool>;
}
