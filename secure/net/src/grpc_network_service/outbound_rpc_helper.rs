// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use tokio::runtime;
use tokio::runtime::Runtime;
use crate::grpc_network_service::GRPCNetworkMessageServiceClientWrapper;
use crate::network_controller::{Message, MessageType};

pub struct OutboundRpcHelper {
    self_addr: SocketAddr,
    outbound_rpc_runtime: Runtime,
    grpc_client: GRPCNetworkMessageServiceClientWrapper
}

impl OutboundRpcHelper {
    pub fn new(self_addr: SocketAddr, remote_addr: SocketAddr) -> Self {
        let outbound_rpc_runtime = runtime::Builder::new_multi_thread().enable_all().thread_name("outbound_rpc_helper").build().unwrap();
        Self {
            self_addr,
            grpc_client: GRPCNetworkMessageServiceClientWrapper::new(&outbound_rpc_runtime, remote_addr),
            outbound_rpc_runtime
        }
    }

    pub fn send(&mut self, msg: Message, mt: &MessageType) {
        self.outbound_rpc_runtime.block_on(async {
            self.grpc_client
                .send_message(self.self_addr, msg, mt).await;
        });
    }
}