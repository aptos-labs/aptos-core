// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Event types for communication between primary and proxy consensus.
//!
//! These are used by the primary RoundManager and proxy RoundManager to
//! exchange QC/TC updates and ordered proxy blocks.

use aptos_consensus_types::{
    proxy_messages::OrderedProxyBlocksMsg, quorum_cert::QuorumCert,
    timeout_2chain::TwoChainTimeoutCertificate,
};
use std::sync::Arc;

/// Events sent from primary RoundManager to proxy RoundManager.
#[derive(Debug)]
pub enum PrimaryToProxyEvent {
    /// New primary QC available - may trigger proxy block "cutting"
    NewPrimaryQC(Arc<QuorumCert>),
    /// New primary TC available - for tracking primary round
    NewPrimaryTC(Arc<TwoChainTimeoutCertificate>),
    /// Shutdown signal
    Shutdown,
}

/// Events sent from proxy RoundManager to primary RoundManager.
#[derive(Debug)]
pub enum ProxyToPrimaryEvent {
    /// Ordered proxy blocks ready to be aggregated into primary block
    OrderedProxyBlocks(OrderedProxyBlocksMsg),
}
