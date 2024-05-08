// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    types::{CertificateAckState, CertifiedNodeMessage, NodeCertificateMessage, SignatureBuilder},
    DAGMessage, DAGRpcResult, Node,
};
use aptos_consensus_types::common::Author;
use aptos_reliable_broadcast::ReliableBroadcast;
use futures::FutureExt;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio_retry::strategy::ExponentialBackoff;

pub enum BoltBCParms {
    Node(Node, Arc<SignatureBuilder>, Vec<Author>),
    CertifiedNode(
        NodeCertificateMessage,
        Arc<CertificateAckState>,
        Vec<Author>,
    ),
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
            BoltBCParms::Node(node, signature_builder, peers) => BoltBCRet::Node(
                async move { rb.multicast(node, signature_builder, peers).await }.boxed(),
            ),
            // BoltBCParms::CertifiedNode(certified_node, ack_status) => BoltBCRet::CertifiedNode(Pin::new(Box::new(rb.broadcast(certified_node, ack_status)))),
            BoltBCParms::CertifiedNode(certified_node, ack_status, peers) => {
                BoltBCRet::CertifiedNode(
                    async move { rb.multicast(certified_node, ack_status, peers).await }.boxed(),
                )
            },
        }
    }
}
