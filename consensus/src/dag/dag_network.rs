use crate::dag::types::DAGMessage;
use aptos_consensus_types::common::Author;
use aptos_network::protocols::network::RpcError;
use async_trait::async_trait;
use std::time::Duration;

pub trait RpcHandler {
    type Ack;
    type Message;

    fn process(&mut self, message: Self::Message) -> anyhow::Result<Self::Ack>;
}

#[async_trait]
pub trait DAGNetworkSender: Send + Sync {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: DAGMessage,
        timeout: Duration,
    ) -> Result<DAGMessage, RpcError>;
}
