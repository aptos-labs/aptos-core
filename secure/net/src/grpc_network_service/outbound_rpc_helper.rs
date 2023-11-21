// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::runtime;
use tokio::runtime::Runtime;
use crate::grpc_network_service::GRPCNetworkMessageServiceClientWrapper;
use crate::network_controller::{Message, MessageType};
use crate::network_controller::metrics::REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER;

pub struct OutboundRpcHelper {
    self_addr: SocketAddr,
    outbound_rpc_runtime: Arc<Runtime>,
    grpc_client: GRPCNetworkMessageServiceClientWrapper
}

impl OutboundRpcHelper {
    pub fn new(self_addr: SocketAddr, remote_addr: SocketAddr, outbound_rpc_runtime: Arc<Runtime>) -> Self {
        Self {
            self_addr,
            grpc_client: GRPCNetworkMessageServiceClientWrapper::new(&outbound_rpc_runtime, remote_addr),
            outbound_rpc_runtime
        }
    }

    pub fn send(&mut self, msg: Message, mt: &MessageType) {
        self.outbound_rpc_runtime.block_on(async {
            if msg.start_ms_since_epoch.is_some() {
                let curr_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
                let mut delta = 0.0;
                if curr_time > msg.start_ms_since_epoch.unwrap() {
                    delta = (curr_time - msg.start_ms_since_epoch.unwrap()) as f64;
                }
                /*if mt.get_type() == "remote_kv_request" {
                    REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER
                        .with_label_values(&["0_kv_req_grpc_shard_send_2_in_async_rt"]).observe(delta);
                } else if mt.get_type() == "remote_kv_response" {
                    REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER
                        .with_label_values(&["5_kv_resp_coord_grpc_send_2_in_async_rt"]).observe(delta);
                }*/
            }
            self.grpc_client
                .send_message(self.self_addr, msg, mt).await;
        });
    }
}
