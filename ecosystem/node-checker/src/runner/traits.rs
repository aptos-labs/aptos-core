// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{configuration::NodeAddress, CheckSummary};
use anyhow::Result;
use async_trait::async_trait;

/// This trait describes a Runner. The Runner is the top level coordinator
/// reponsible for driving a check. It is expected to take in the information
/// of a target node, build whatever Providers it can based on the request,
/// run the configured Checkers, and finally return a CheckSummary.
///
/// Note on trait bounds:
///  - Sync + Send is required because this will be a member of the Api which
///    needs to be used across thread boundaries.
///  - The 'static lifetime is required because this will be stored on the Api
///    which needs to be 'static.
#[async_trait]
pub trait Runner: Sync + Send + 'static {
    async fn run(&self, target_node_address: &NodeAddress) -> Result<CheckSummary>;
}
