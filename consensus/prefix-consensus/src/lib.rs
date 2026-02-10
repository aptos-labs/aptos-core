// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Aptos Prefix Consensus
//!
//! This crate implements the primitive 3-round asynchronous Prefix Consensus protocol
//! as described in the paper "Prefix Consensus For Censorship Resistant BFT".
//!
//! ## Overview
//!
//! Prefix Consensus is a consensus primitive where parties propose vectors of values
//! and output compatible vectors extending the maximum common prefix of honest inputs.
//!
//! Unlike traditional consensus:
//! - Does NOT require agreement on single output value
//! - CAN be solved deterministically in asynchronous setting
//! - Outputs two values: v_low (safe to commit) and v_high (safe to extend)
//!
//! ## Protocol
//!
//! The protocol runs in 3 rounds:
//! 1. **Round 1**: Vote on input vectors, certify longest prefix with >1/3 stake
//! 2. **Round 2**: Vote on certified prefixes, compute maximum common prefix
//! 3. **Round 3**: Vote on round-2 prefixes, output (v_low, v_high)
//!
//! Quorum thresholds use proof-of-stake weighted voting:
//! - QC formation requires >2/3 of total stake
//! - Certified prefix requires >1/3 of total stake (minority threshold)
//!
//! ## Properties
//!
//! - **Upper Bound**: v_low_i ⪯ v_high_j for any honest parties i,j
//! - **Termination**: Every honest party eventually outputs
//! - **Validity**: mcp({v_in_h}_{h∈H}) ⪯ v_low_i for any honest party i

pub mod certificates;
mod certify;
pub mod manager;
pub mod network_interface;
pub mod network_messages;
mod protocol;
pub mod signing;
pub mod strong_protocol;
mod types;
mod utils;
mod verification;
pub mod view_state;

pub use certify::{qc1_certify, qc2_certify, qc3_certify};
pub use manager::PrefixConsensusManager;
pub use network_interface::{
    NetworkSenderAdapter, PrefixConsensusNetworkClient, PrefixConsensusNetworkSender,
};
pub use network_messages::PrefixConsensusMsg;
pub use protocol::PrefixConsensusProtocol;
pub use signing::{
    sign_vote1, sign_vote2, sign_vote3, verify_vote1_signature, verify_vote2_signature,
    verify_vote3_signature,
};
pub use types::{
    Element, PartyId, PrefixConsensusInput, PrefixConsensusOutput, PrefixVector, Round, Vote1,
    Vote2, Vote3, QC1, QC2, QC3,
};
pub use utils::{consistency_check, first_non_bot, max_common_prefix, min_common_extension};
pub use verification::{
    qc1_view, qc2_view, qc3_view, verify_qc1, verify_qc2, verify_qc3, verify_vote1, verify_vote2,
    verify_vote3,
};
pub use certificates::{
    cert_reaches_view1, Certificate, DirectCertificate, EmptyViewMessage, EmptyViewStatement,
    HighestKnownView, IndirectCertificate, StrongPCCommit, StrongPCCommitError,
};
pub use strong_protocol::{
    ChainBuildError, StrongPrefixConsensusProtocol, View1Decision, ViewDecision,
};
pub use view_state::{
    has_non_bot_entry, RankingManager, ViewOutput,
    ViewState,
};
