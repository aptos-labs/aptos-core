// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Main Prefix Consensus Protocol implementation
//!
//! This module implements the 3-round asynchronous Prefix Consensus protocol
//! as described in Algorithm 1 of the paper.

use crate::{
    certify::{qc1_certify, qc2_certify, qc3_certify},
    types::{
        PendingVotes1, PendingVotes2, PendingVotes3, PrefixConsensusInput,
        PrefixConsensusOutput, PrefixVector, Vote1, Vote2, Vote3, QC1, QC2, QC3,
    },
    verification::{verify_qc1, verify_qc2, verify_qc3, verify_vote1, verify_vote2, verify_vote3},
};
use anyhow::{bail, Result};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_logger::{debug, error, info};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The state of the protocol
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolState {
    /// Initial state, not started
    NotStarted,

    /// Round 1: Voting on inputs, collecting votes
    Round1,

    /// Round 1 complete, QC1 formed
    Round1Complete,

    /// Round 2: Voting on certified prefixes
    Round2,

    /// Round 2 complete, QC2 formed
    Round2Complete,

    /// Round 3: Voting on mcp prefixes
    Round3,

    /// Protocol complete, output available
    Complete,
}

/// Main Prefix Consensus Protocol implementation
///
/// This struct manages the state and execution of the 3-round protocol.
pub struct PrefixConsensusProtocol {
    /// Input to the protocol
    input: PrefixConsensusInput,

    /// Current protocol state
    state: Arc<RwLock<ProtocolState>>,

    /// Pending Vote1 messages
    pending_votes1: Arc<RwLock<PendingVotes1>>,

    /// Pending Vote2 messages
    pending_votes2: Arc<RwLock<PendingVotes2>>,

    /// Pending Vote3 messages
    pending_votes3: Arc<RwLock<PendingVotes3>>,

    /// QC1 after Round 1
    qc1: Arc<RwLock<Option<QC1>>>,

    /// Certified prefix extracted from QC1
    certified_prefix: Arc<RwLock<Option<PrefixVector>>>,

    /// QC2 after Round 2
    qc2: Arc<RwLock<Option<QC2>>>,

    /// MCP prefix from QC2
    mcp_prefix: Arc<RwLock<Option<PrefixVector>>>,

    /// QC3 after Round 3
    qc3: Arc<RwLock<Option<QC3>>>,

    /// Final output
    output: Arc<RwLock<Option<PrefixConsensusOutput>>>,
}

impl PrefixConsensusProtocol {
    /// Create a new protocol instance
    pub fn new(input: PrefixConsensusInput) -> Self {
        Self {
            input,
            state: Arc::new(RwLock::new(ProtocolState::NotStarted)),
            pending_votes1: Arc::new(RwLock::new(PendingVotes1::new())),
            pending_votes2: Arc::new(RwLock::new(PendingVotes2::new())),
            pending_votes3: Arc::new(RwLock::new(PendingVotes3::new())),
            qc1: Arc::new(RwLock::new(None)),
            certified_prefix: Arc::new(RwLock::new(None)),
            qc2: Arc::new(RwLock::new(None)),
            mcp_prefix: Arc::new(RwLock::new(None)),
            qc3: Arc::new(RwLock::new(None)),
            output: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the current protocol state
    pub async fn get_state(&self) -> ProtocolState {
        self.state.read().await.clone()
    }

    /// Get the final output (if protocol is complete)
    pub async fn get_output(&self) -> Option<PrefixConsensusOutput> {
        self.output.read().await.clone()
    }

    // ========================================================================
    // Round 1: Voting on inputs
    // ========================================================================

    /// Start Round 1: Broadcast Vote1 with input vector
    pub async fn start_round1(&self, private_key: &Ed25519PrivateKey) -> Result<Vote1> {
        let mut state = self.state.write().await;
        if *state != ProtocolState::NotStarted {
            bail!("Cannot start Round 1: protocol already started");
        }

        info!(
            party_id = %self.input.party_id,
            input_len = self.input.input_vector.len(),
            "Starting Round 1"
        );

        *state = ProtocolState::Round1;
        drop(state);

        // Create Vote1
        // For the prototype, we create a dummy signature
        // In production, this would properly sign the vote hash with proper message types
        let signature = create_dummy_signature(private_key);
        let vote = Vote1::new(
            self.input.party_id,
            self.input.input_vector.clone(),
            signature,
        );

        // Add own vote to pending votes
        self.process_vote1(vote.clone()).await?;

        Ok(vote)
    }

    /// Process an incoming Vote1
    pub async fn process_vote1(&self, vote: Vote1) -> Result<Option<QC1>> {
        let state = self.state.read().await.clone();
        if state != ProtocolState::Round1 {
            bail!("Not in Round 1, ignoring Vote1");
        }
        drop(state);

        // Verify vote
        verify_vote1(&vote)?;

        debug!(
            author = %vote.author,
            vector_len = vote.input_vector.len(),
            "Processing Vote1"
        );

        // Add to pending votes
        let mut pending = self.pending_votes1.write().await;
        if !pending.add_vote(vote) {
            // Duplicate vote
            return Ok(None);
        }

        let vote_count = pending.vote_count();
        let quorum_size = self.input.quorum_size();

        info!(
            vote_count = vote_count,
            quorum_size = quorum_size,
            "Vote1 count updated"
        );

        // Check if we have quorum
        if pending.has_quorum(quorum_size) {
            info!("Quorum reached for Round 1, forming QC1");

            // Form QC1
            let votes = pending.votes().values().cloned().collect();
            let qc1 = QC1::new(votes);

            // Verify QC1
            verify_qc1(&qc1, self.input.f, self.input.n)?;

            // Store QC1
            *self.qc1.write().await = Some(qc1.clone());

            // Extract certified prefix
            let certified = qc1_certify(&qc1, self.input.f);
            info!(certified_len = certified.len(), "Extracted certified prefix");

            *self.certified_prefix.write().await = Some(certified);

            // Update state
            *self.state.write().await = ProtocolState::Round1Complete;

            return Ok(Some(qc1));
        }

        Ok(None)
    }

    // ========================================================================
    // Round 2: Voting on certified prefixes
    // ========================================================================

    /// Start Round 2: Broadcast Vote2 with certified prefix from QC1
    pub async fn start_round2(&self, private_key: &Ed25519PrivateKey) -> Result<Vote2> {
        let mut state = self.state.write().await;
        if *state != ProtocolState::Round1Complete {
            bail!("Cannot start Round 2: Round 1 not complete");
        }

        *state = ProtocolState::Round2;
        drop(state);

        let qc1 = self
            .qc1
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("QC1 not available"))?;

        let certified = self
            .certified_prefix
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Certified prefix not available"))?;

        info!(
            party_id = %self.input.party_id,
            certified_len = certified.len(),
            "Starting Round 2"
        );

        // Create Vote2
        // For the prototype, we create a dummy signature
        let signature = create_dummy_signature(private_key);
        let vote = Vote2::new(self.input.party_id, certified.clone(), qc1, signature);

        // Add own vote to pending votes
        self.process_vote2(vote.clone()).await?;

        Ok(vote)
    }

    /// Process an incoming Vote2
    pub async fn process_vote2(&self, vote: Vote2) -> Result<Option<QC2>> {
        let state = self.state.read().await.clone();
        if state != ProtocolState::Round2 {
            bail!("Not in Round 2, ignoring Vote2");
        }
        drop(state);

        // Verify vote
        verify_vote2(&vote, self.input.f, self.input.n)?;

        debug!(
            author = %vote.author,
            prefix_len = vote.certified_prefix.len(),
            "Processing Vote2"
        );

        // Add to pending votes
        let mut pending = self.pending_votes2.write().await;
        if !pending.add_vote(vote) {
            return Ok(None);
        }

        let vote_count = pending.vote_count();
        let quorum_size = self.input.quorum_size();

        info!(
            vote_count = vote_count,
            quorum_size = quorum_size,
            "Vote2 count updated"
        );

        // Check if we have quorum
        if pending.has_quorum(quorum_size) {
            info!("Quorum reached for Round 2, forming QC2");

            // Form QC2
            let votes = pending.votes().values().cloned().collect();
            let qc2 = QC2::new(votes);

            // Verify QC2
            verify_qc2(&qc2, self.input.f, self.input.n)?;

            // Store QC2
            *self.qc2.write().await = Some(qc2.clone());

            // Compute mcp
            let mcp = qc2_certify(&qc2);
            info!(mcp_len = mcp.len(), "Computed MCP from QC2");

            *self.mcp_prefix.write().await = Some(mcp);

            // Update state
            *self.state.write().await = ProtocolState::Round2Complete;

            return Ok(Some(qc2));
        }

        Ok(None)
    }

    // ========================================================================
    // Round 3: Voting on mcp prefixes
    // ========================================================================

    /// Start Round 3: Broadcast Vote3 with mcp prefix from QC2
    pub async fn start_round3(&self, private_key: &Ed25519PrivateKey) -> Result<Vote3> {
        let mut state = self.state.write().await;
        if *state != ProtocolState::Round2Complete {
            bail!("Cannot start Round 3: Round 2 not complete");
        }

        *state = ProtocolState::Round3;
        drop(state);

        let qc2 = self
            .qc2
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("QC2 not available"))?;

        let mcp = self
            .mcp_prefix
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("MCP prefix not available"))?;

        info!(
            party_id = %self.input.party_id,
            mcp_len = mcp.len(),
            "Starting Round 3"
        );

        // Create Vote3
        // For the prototype, we create a dummy signature
        let signature = create_dummy_signature(private_key);
        let vote = Vote3::new(self.input.party_id, mcp.clone(), qc2, signature);

        // Add own vote to pending votes
        self.process_vote3(vote.clone()).await?;

        Ok(vote)
    }

    /// Process an incoming Vote3
    pub async fn process_vote3(&self, vote: Vote3) -> Result<Option<PrefixConsensusOutput>> {
        let state = self.state.read().await.clone();
        if state != ProtocolState::Round3 {
            bail!("Not in Round 3, ignoring Vote3");
        }
        drop(state);

        // Verify vote
        verify_vote3(&vote, self.input.f, self.input.n)?;

        debug!(
            author = %vote.author,
            prefix_len = vote.mcp_prefix.len(),
            "Processing Vote3"
        );

        // Add to pending votes
        let mut pending = self.pending_votes3.write().await;
        if !pending.add_vote(vote) {
            return Ok(None);
        }

        let vote_count = pending.vote_count();
        let quorum_size = self.input.quorum_size();

        info!(
            vote_count = vote_count,
            quorum_size = quorum_size,
            "Vote3 count updated"
        );

        // Check if we have quorum
        if pending.has_quorum(quorum_size) {
            info!("Quorum reached for Round 3, forming QC3 and computing output");

            // Form QC3
            let votes = pending.votes().values().cloned().collect();
            let qc3 = QC3::new(votes);

            // Verify QC3
            verify_qc3(&qc3, self.input.f, self.input.n)?;

            // Store QC3
            *self.qc3.write().await = Some(qc3.clone());

            // Compute final output
            let (v_low, v_high) = qc3_certify(&qc3);

            info!(
                v_low_len = v_low.len(),
                v_high_len = v_high.len(),
                "Protocol complete, output computed"
            );

            let output = PrefixConsensusOutput::new(v_low, v_high, qc3);

            // Verify upper bound property
            if !output.verify_upper_bound() {
                error!("Output violates upper bound property!");
                bail!("Output violates upper bound: v_low is not a prefix of v_high");
            }

            // Store output
            *self.output.write().await = Some(output.clone());

            // Update state
            *self.state.write().await = ProtocolState::Complete;

            return Ok(Some(output));
        }

        Ok(None)
    }

    // ========================================================================
    // Utility methods
    // ========================================================================

    /// Get current vote counts for all rounds
    pub async fn get_vote_counts(&self) -> (usize, usize, usize) {
        let count1 = self.pending_votes1.read().await.vote_count();
        let count2 = self.pending_votes2.read().await.vote_count();
        let count3 = self.pending_votes3.read().await.vote_count();
        (count1, count2, count3)
    }

    /// Check if protocol is complete
    pub async fn is_complete(&self) -> bool {
        *self.state.read().await == ProtocolState::Complete
    }

    /// Get QC1 if available
    pub async fn get_qc1(&self) -> Option<QC1> {
        self.qc1.read().await.clone()
    }

    /// Get QC2 if available
    pub async fn get_qc2(&self) -> Option<QC2> {
        self.qc2.read().await.clone()
    }

    /// Get QC3 if available
    pub async fn get_qc3(&self) -> Option<QC3> {
        self.qc3.read().await.clone()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a dummy signature for the prototype
///
/// Note: This is a placeholder for the prototype. In production, votes would be
/// properly signed using the private key with the correct message format.
fn create_dummy_signature(_private_key: &Ed25519PrivateKey) -> Ed25519Signature {
    // Create a dummy 64-byte signature
    // In a real implementation, this would properly sign the vote content
    Ed25519Signature::try_from(&[0u8; 64][..]).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey};

    fn hash(i: u64) -> HashValue {
        HashValue::sha3_256_of(&i.to_le_bytes())
    }

    fn create_test_input(
        party_id: u8,
        vector: PrefixVector,
        n: usize,
        f: usize,
    ) -> PrefixConsensusInput {
        PrefixConsensusInput::new(vector, PartyId::new([party_id; 32]), n, f)
    }

    #[tokio::test]
    async fn test_protocol_state_transitions() {
        let input = create_test_input(0, vec![hash(1), hash(2)], 4, 1);
        let protocol = PrefixConsensusProtocol::new(input);

        assert_eq!(protocol.get_state().await, ProtocolState::NotStarted);

        let private_key = Ed25519PrivateKey::generate_for_testing();
        protocol.start_round1(&private_key).await.unwrap();

        assert_eq!(protocol.get_state().await, ProtocolState::Round1);
    }

    #[tokio::test]
    async fn test_round1_vote_collection() {
        let input = create_test_input(0, vec![hash(1), hash(2)], 4, 1);
        let protocol = PrefixConsensusProtocol::new(input);

        let private_key = Ed25519PrivateKey::generate_for_testing();
        protocol.start_round1(&private_key).await.unwrap();

        // Need 3 votes for quorum (n=4, f=1, quorum=3)
        let (count1, _, _) = protocol.get_vote_counts().await;
        assert_eq!(count1, 1); // Own vote added
    }
}
