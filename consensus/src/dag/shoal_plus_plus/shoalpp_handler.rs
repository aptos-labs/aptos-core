// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::network::{IncomingShoalppRequest, IncomingDAGRequest};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_logger::{debug, error};
use aptos_types::epoch_state::EpochState;
use futures::StreamExt;
use futures_channel::oneshot;
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
        mut shoalpp_rpc_rx: aptos_channel::Receiver<Author, (AccountAddress, IncomingShoalppRequest)>,
        mut shutdown_rx: oneshot::Receiver<oneshot::Sender<()>>,
        dag_rpc_tx_vec: Vec<aptos_channel::Sender<AccountAddress, IncomingDAGRequest>>,
        mut dag_shutdown_tx_vec: Vec<oneshot::Sender<oneshot::Sender<()>>>,
    ) {
        loop {
            tokio::select! {
                biased;
                Ok(ack_tx) = &mut shutdown_rx => {
                    while !dag_shutdown_tx_vec.is_empty() {
                        let (ack_tx, ack_rx) = oneshot::channel();
                        dag_shutdown_tx_vec
                            .pop()
                            .unwrap()
                            .send(ack_tx)
                            .expect("[BoltHandler] Fail to drop DAG bootstrapper");
                        ack_rx
                            .await
                            .expect("[BoltHandler] Fail to drop DAG bootstrapper");
                    }
                    if let Err(e) = ack_tx.send(()) {
                        error!(error = ?e, "unable to ack to shutdown signal");
                    }
                    return;
                },

                (peer_id, msg) = shoalpp_rpc_rx.select_next_some() => {
                    match self.convert(msg) {
                        Ok(dag_req) => {
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
        dag_req
            .req
            .verify(dag_req.sender, &self.epoch_state.verifier)?;
        Ok(dag_req)
    }
}