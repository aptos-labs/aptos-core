// Copyright © Aptos Foundation

use crate::network_interface::ConsensusMsg;
use aptos_consensus_types::common::Author;
use async_trait::async_trait;
use std::time::Duration;

pub trait DKGRpcHandler {
    type DKGRequest;
    type DKGResponse;

    fn process(&mut self, message: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse>;
}

#[async_trait]
pub trait DKGNetworkSender: Send + Sync {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg>;

    /// Given a list of potential responders, sending rpc to get response from any of them and could
    /// fallback to more in case of failures.
    async fn send_rpc_with_fallbacks(
        &self,
        responders: Vec<Author>,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg>;
}
