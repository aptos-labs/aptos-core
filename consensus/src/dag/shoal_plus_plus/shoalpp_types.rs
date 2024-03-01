// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{types::{CertificateAckState, CertifiedNodeMessage, SignatureBuilder}, DAGMessage, Node, DAGRpcResult};
use aptos_reliable_broadcast::ReliableBroadcast;
use futures::FutureExt;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio_retry::strategy::ExponentialBackoff;

pub enum BoltBCParms {
    Node(Node, Arc<SignatureBuilder>),
    CertifiedNode(CertifiedNodeMessage, Arc<CertificateAckState>),
}

pub enum BoltBCRet {
    Node(Pin<Box<dyn Future<Output = ()> + Send>>),
    CertifiedNode(Pin<Box<dyn Future<Output = ()> + Send>>),
}

impl BoltBCParms {
    pub fn broadcast(
        self,
        rb: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff, DAGRpcResult>>,
    ) -> BoltBCRet {
        match self {
            // BoltBCParms::Node(node, signature_builder) => BoltBCRet::Node(Pin::new(Box::new(rb.broadcast(node, signature_builder)))),
            BoltBCParms::Node(node, signature_builder) => {
                BoltBCRet::Node(async move { rb.broadcast(node, signature_builder).await }.boxed())
            },
            // BoltBCParms::CertifiedNode(certified_node, ack_status) => BoltBCRet::CertifiedNode(Pin::new(Box::new(rb.broadcast(certified_node, ack_status)))),
            BoltBCParms::CertifiedNode(certified_node, ack_status) => BoltBCRet::CertifiedNode(
                async move { rb.broadcast(certified_node, ack_status).await }.boxed(),
            ),
        }
    }
}