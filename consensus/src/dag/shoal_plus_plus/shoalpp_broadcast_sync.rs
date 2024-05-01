// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    shoal_plus_plus::shoalpp_types::{BoltBCParms, BoltBCRet},
    DAGMessage, DAGRpcResult,
};
use aptos_logger::debug;
use aptos_reliable_broadcast::ReliableBroadcast;
use async_trait::async_trait;
use futures_channel::oneshot;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio_retry::strategy::ExponentialBackoff;

#[async_trait]
pub trait BroadcastSync {
    async fn run(self);
}

pub struct BroadcastNoSync {
    reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff, DAGRpcResult>>,
    receivers: Vec<Receiver<(oneshot::Sender<BoltBCRet>, BoltBCParms)>>,
}

impl BroadcastNoSync {
    pub fn new(
        reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff, DAGRpcResult>>,
        receivers: Vec<Receiver<(oneshot::Sender<BoltBCRet>, BoltBCParms)>>,
    ) -> Self {
        Self {
            reliable_broadcast,
            receivers,
        }
    }
}

#[async_trait]
impl BroadcastSync for BroadcastNoSync {
    async fn run(mut self) {
        assert_eq!(self.receivers.len(), 1);

        // TODO: shutdown mechanism
        loop {
            let (ret_tx1, bolt_bc_parms) = self.receivers[0].recv().await.unwrap();
            if let Err(_e) = ret_tx1.send(bolt_bc_parms.broadcast(self.reliable_broadcast.clone()))
            {
                // TODO: should we panic here?
            }
        }
    }
}

// TODO: handle the Bolt disabled case

pub struct BoltBroadcastSync {
    reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff, DAGRpcResult>>,
    receivers: Vec<Receiver<(oneshot::Sender<BoltBCRet>, BoltBCParms)>>,
}

impl BoltBroadcastSync {
    pub fn new(
        reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff, DAGRpcResult>>,
        receivers: Vec<Receiver<(oneshot::Sender<BoltBCRet>, BoltBCParms)>>,
    ) -> Self {
        Self {
            reliable_broadcast,
            receivers,
        }
    }
}

#[async_trait]
impl BroadcastSync for BoltBroadcastSync {
    async fn run(mut self) {
        assert_eq!(self.receivers.len(), 3);
        // TODO: think about synchronization after state sync.

        loop {
            for i in 0..3 {
                // TODO: think about the unwrap()
                let (ret_tx, bolt_bc_parms) = self.receivers[i].recv().await.unwrap();
                if let Err(_e) =
                    ret_tx.send(bolt_bc_parms.broadcast(self.reliable_broadcast.clone()))
                {
                    // TODO: should we panic here?
                }
            }
        }
    }
}
