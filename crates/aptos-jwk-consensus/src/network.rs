// Copyright © Aptos Foundation

use crate::types::JWKConsensusMsg;
use aptos_infallible::RwLock;
use aptos_types::account_address::AccountAddress;
use std::sync::Arc;

pub struct IncomingRpcRequest {
    pub msg: JWKConsensusMsg,
    pub sender: AccountAddress,
    pub response_sender: Box<dyn RpcResponseSender>,
}

pub trait RpcResponseSender: Send + Sync {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>);
}

pub struct DummyRpcResponseSender {
    pub rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<JWKConsensusMsg>>>>,
}

impl DummyRpcResponseSender {
    pub fn new(rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<JWKConsensusMsg>>>>) -> Self {
        Self {
            rpc_response_collector,
        }
    }
}

impl RpcResponseSender for DummyRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>) {
        self.rpc_response_collector.write().push(response);
    }
}
