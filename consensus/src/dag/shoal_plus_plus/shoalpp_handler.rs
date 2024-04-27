// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::observability::counters::RPC_PROCESS_DURATION,
    network::{IncomingDAGRequest, IncomingShoalppRequest},
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_logger::debug;
use aptos_types::epoch_state::EpochState;
use futures::StreamExt;
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

pub(crate) struct BoltHandler {
    epoch_state: Arc<EpochState>,
}

impl BoltHandler {
    pub fn new(epoch_state: Arc<EpochState>) -> Self {
        Self { epoch_state }
    }

    pub async fn run(
        self,
        // mut shoalpp_rpc_rx: aptos_channel::Receiver<Author, (AccountAddress, IncomingShoalppRequest)>,
        // mut shutdown_rx: oneshot::Receiver<oneshot::Sender<()>>,
        mut shoalpp_rpc_rx: aptos_channel::Receiver<
            Author,
            (AccountAddress, IncomingShoalppRequest),
        >,
        dag_rpc_tx_vec: Vec<aptos_channel::Sender<AccountAddress, IncomingDAGRequest>>,
        // mut dag_shutdown_tx_vec: Vec<oneshot::Sender<oneshot::Sender<()>>>,
    ) {
        loop {
            tokio::select! {

                (peer_id, msg) = shoalpp_rpc_rx.select_next_some() => {
                    let start = msg.start;
                    RPC_PROCESS_DURATION
                        .with_label_values(&["shoalpp_handler"])
                        .observe(start.elapsed().as_secs_f64());
                    match self.convert(msg) {
                        Ok(dag_req) => {
                            RPC_PROCESS_DURATION
                                .with_label_values(&["shoalpp_convert"])
                                .observe(start.elapsed().as_secs_f64());
                            let dag_id = dag_req.dag_id();
                            if let Err(e) = dag_rpc_tx_vec[dag_id as usize].push(peer_id, dag_req){
                                debug!("failed to push req to dag {}: {}", dag_id, e);
                            }

                        }
                        Err(e) => {
                            debug!("bad BoltReq:, {}", e);
                        }
                    }
                }


            }
        }
    }

    fn convert(&self, bolt_req: IncomingShoalppRequest) -> anyhow::Result<IncomingDAGRequest> {
        let dag_req: IncomingDAGRequest = bolt_req.try_into()?;
        Ok(dag_req)
    }
}
