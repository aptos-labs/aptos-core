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
//! 1. **Round 1**: Vote on input vectors, certify longest prefix in f+1 votes
//! 2. **Round 2**: Vote on certified prefixes, compute maximum common prefix
//! 3. **Round 3**: Vote on round-2 prefixes, output (v_low, v_high)
//!
//! ## Properties
//!
//! - **Upper Bound**: v_low_i ⪯ v_high_j for any honest parties i,j
//! - **Termination**: Every honest party eventually outputs
//! - **Validity**: mcp({v_in_h}_{h∈H}) ⪯ v_low_i for any honest party i

mod certify;
mod protocol;
mod types;
mod utils;
mod verification;

pub use certify::{qc1_certify, qc2_certify, qc3_certify};
pub use protocol::PrefixConsensusProtocol;
pub use types::{
    PartyId, PrefixConsensusInput, PrefixConsensusOutput, Round, Vote1, Vote2, Vote3, QC1, QC2,
    QC3,
};
pub use utils::{consistency_check, max_common_prefix, min_common_extension};
pub use verification::{verify_qc1, verify_qc2, verify_qc3, verify_vote1, verify_vote2, verify_vote3};
