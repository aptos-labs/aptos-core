// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Prefix Consensus Manager
//!
//! This module provides the event-driven manager that orchestrates the Prefix Consensus
//! protocol lifecycle. It handles incoming network messages, verifies signatures,
//! progresses through protocol rounds, and triggers vote broadcasts.

use crate::{
    network_interface::PrefixConsensusNetworkSender,
    network_messages::PrefixConsensusMsg,
    protocol::PrefixConsensusProtocol,
    signing::{verify_vote1_signature, verify_vote2_signature, verify_vote3_signature},
    types::{PartyId, PrefixConsensusOutput, Vote1, Vote2, Vote3},
};
use anyhow::Result;
use aptos_consensus_types::common::Author;
use aptos_logger::prelude::*;
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use futures::{FutureExt, StreamExt};
use std::{
    collections::HashSet,
    sync::Arc,
};
use tokio::sync::RwLock;

/// Manager for Prefix Consensus protocol execution
///
/// Orchestrates the protocol lifecycle: receives network messages, verifies signatures,
/// passes votes to the protocol, and broadcasts new votes as rounds progress.
pub struct PrefixConsensusManager<NetworkSender> {
    /// This party's ID
    party_id: PartyId,

    /// Current epoch
    epoch: u64,

    /// The underlying protocol state machine
    protocol: Arc<PrefixConsensusProtocol>,

    /// Network sender for broadcasting votes
    network_sender: NetworkSender,

    /// Validator signer for signature verification
    validator_signer: ValidatorSigner,

    /// Validator verifier for signature checking
    validator_verifier: Arc<ValidatorVerifier>,

    /// Track seen Vote1 messages to prevent duplicates
    seen_vote1: Arc<RwLock<HashSet<PartyId>>>,

    /// Track seen Vote2 messages to prevent duplicates
    seen_vote2: Arc<RwLock<HashSet<PartyId>>>,

    /// Track seen Vote3 messages to prevent duplicates
    seen_vote3: Arc<RwLock<HashSet<PartyId>>>,
}

impl<NetworkSender: PrefixConsensusNetworkSender> PrefixConsensusManager<NetworkSender> {
    /// Create a new Prefix Consensus manager
    pub fn new(
        party_id: PartyId,
        epoch: u64,
        protocol: Arc<PrefixConsensusProtocol>,
        network_sender: NetworkSender,
        validator_signer: ValidatorSigner,
        validator_verifier: Arc<ValidatorVerifier>,
    ) -> Self {
        Self {
            party_id,
            epoch,
            protocol,
            network_sender,
            validator_signer,
            validator_verifier,
            seen_vote1: Arc::new(RwLock::new(HashSet::new())),
            seen_vote2: Arc::new(RwLock::new(HashSet::new())),
            seen_vote3: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Initialize the protocol
    ///
    /// This should be called after creating the manager and before starting the event loop.
    /// Unlike AptosBFT's RoundManager::init(), this doesn't need to process existing state
    /// since Prefix Consensus is a single-shot protocol.
    pub async fn init(&self) -> Result<()> {
        info!(
            party_id = %self.party_id,
            epoch = self.epoch,
            validator_count = self.validator_verifier.len(),
            total_stake = self.validator_verifier.total_voting_power(),
            "Initializing Prefix Consensus"
        );

        Ok(())
    }

    /// Main event loop for processing incoming Prefix Consensus messages
    ///
    /// Consumes self and runs until either:
    /// - The protocol completes (QC3 forms)
    /// - A shutdown signal is received via close_rx
    ///
    /// This follows the RoundManager pattern from AptosBFT consensus.
    pub async fn run(
        self,
        mut message_rx: aptos_channels::UnboundedReceiver<(Author, PrefixConsensusMsg)>,
        close_rx: futures::channel::oneshot::Receiver<futures::channel::oneshot::Sender<()>>,
    ) {
        info!(
            party_id = %self.party_id,
            epoch = self.epoch,
            "PrefixConsensusManager event loop started"
        );

        // Broadcast Vote1 FIRST before entering message loop
        // This ensures the receiver is ready before our own Vote1 arrives via self-send
        match self.protocol.start_round1(&self.validator_signer).await {
            Ok((vote1, qc1)) => {
                info!(
                    party_id = %self.party_id,
                    vote_author = %vote1.author,
                    input_len = vote1.input_vector.len(),
                    "Broadcasting Vote1"
                );
                self.network_sender.broadcast_vote1(vote1).await;
                if qc1.is_some() {
                    if let Err(e) = self.start_round2().await {
                        error!(party_id = %self.party_id, error = ?e, "Failed to start Round 2 after early QC1");
                    }
                }
            }
            Err(e) => {
                error!(
                    party_id = %self.party_id,
                    error = ?e,
                    "Failed to start Round 1"
                );
                return;
            }
        }

        let mut close_rx = close_rx.into_stream();

        loop {
            tokio::select! {
                biased;

                // Handle shutdown signal
                close_req = close_rx.select_next_some() => {
                    info!(
                        party_id = %self.party_id,
                        "Received shutdown signal"
                    );
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).expect("[PrefixConsensusManager] Failed to ack shutdown");
                    }
                    break;
                }

                // Handle incoming messages
                Some((author, msg)) = message_rx.next() => {
                    if let Err(e) = self.process_message(author, msg).await {
                        warn!(
                            party_id = %self.party_id,
                            error = ?e,
                            "Failed to process message"
                        );
                    }

                    // Check if protocol is complete
                    if self.is_complete().await {
                        info!(
                            party_id = %self.party_id,
                            "Prefix Consensus protocol complete"
                        );
                        break;
                    }
                }
            }
        }

        info!(
            party_id = %self.party_id,
            "PrefixConsensusManager event loop terminated"
        );
    }

    /// Process an incoming network message
    ///
    /// Routes the message to the appropriate handler based on its type.
    /// Invalid messages (wrong epoch, duplicate, bad signature) are logged and ignored.
    pub async fn process_message(&self, author: Author, msg: PrefixConsensusMsg) -> Result<()> {
        // Check epoch first
        if msg.epoch() != self.epoch {
            warn!(
                party_id = %self.party_id,
                msg_epoch = msg.epoch(),
                expected_epoch = self.epoch,
                msg_type = msg.name(),
                "Ignoring message from wrong epoch"
            );
            return Ok(());
        }

        match msg {
            PrefixConsensusMsg::Vote1Msg(vote) => {
                self.process_vote1(author, *vote).await
            },
            PrefixConsensusMsg::Vote2Msg(vote) => {
                self.process_vote2(author, *vote).await
            },
            PrefixConsensusMsg::Vote3Msg(vote) => {
                self.process_vote3(author, *vote).await
            },
        }
    }

    /// Process a Vote1 message
    async fn process_vote1(&self, author: Author, vote: Vote1) -> Result<()> {
        debug!(
            party_id = %self.party_id,
            vote_author = %vote.author,
            input_len = vote.input_vector.len(),
            "Processing Vote1"
        );

        // Check that claimed author matches network sender
        if vote.author != author {
            warn!(
                party_id = %self.party_id,
                claimed_author = %vote.author,
                actual_sender = %author,
                "Vote1 author mismatch - rejecting potential impersonation attack"
            );
            return Ok(());
        }

        // Check for duplicate
        {
            let mut seen = self.seen_vote1.write().await;
            if seen.contains(&vote.author) {
                debug!(
                    party_id = %self.party_id,
                    vote_author = %vote.author,
                    "Ignoring duplicate Vote1"
                );
                return Ok(());
            }
            seen.insert(vote.author);
        }

        // Verify signature
        if let Err(e) = verify_vote1_signature(&vote, &author, &self.validator_verifier) {
            warn!(
                party_id = %self.party_id,
                vote_author = %vote.author,
                error = ?e,
                "Vote1 signature verification failed"
            );
            return Ok(());
        }

        // Pass to protocol
        match self.protocol.process_vote1(vote).await {
            Ok(Some(qc1)) => {
                info!(
                    party_id = %self.party_id,
                    qc_size = qc1.votes.len(),
                    "QC1 formed, starting Round 2"
                );
                self.start_round2().await?;
            },
            Ok(None) => {
                // Vote processed, but QC not yet formed
                debug!(
                    party_id = %self.party_id,
                    "Vote1 processed, waiting for more votes"
                );
            },
            Err(e) => {
                warn!(
                    party_id = %self.party_id,
                    error = ?e,
                    "Failed to process Vote1"
                );
            },
        }

        Ok(())
    }

    /// Start Round 2 after QC1 formation
    async fn start_round2(&self) -> Result<()> {
        info!(
            party_id = %self.party_id,
            "Starting Round 2"
        );

        // Protocol creates Vote2 (QC1 is already stored in protocol)
        let (vote2, qc2) = self.protocol.start_round2(&self.validator_signer).await?;

        info!(
            party_id = %self.party_id,
            vote_author = %vote2.author,
            certified_prefix_len = vote2.certified_prefix.len(),
            "Broadcasting Vote2"
        );

        // Broadcast our Vote2
        self.network_sender.broadcast_vote2(vote2).await;

        // If QC2 formed during self-vote processing (early votes accumulated),
        // immediately start Round 3
        if qc2.is_some() {
            self.start_round3().await?;
        }

        Ok(())
    }

    /// Process a Vote2 message
    async fn process_vote2(&self, author: Author, vote: Vote2) -> Result<()> {
        debug!(
            party_id = %self.party_id,
            vote_author = %vote.author,
            certified_prefix_len = vote.certified_prefix.len(),
            "Processing Vote2"
        );

        // Check for duplicate
        {
            let mut seen = self.seen_vote2.write().await;
            if seen.contains(&vote.author) {
                debug!(
                    party_id = %self.party_id,
                    vote_author = %vote.author,
                    "Ignoring duplicate Vote2"
                );
                return Ok(());
            }
            seen.insert(vote.author);
        }

        // Check that claimed author matches network sender
        if vote.author != author {
            warn!(
                party_id = %self.party_id,
                claimed_author = %vote.author,
                actual_sender = %author,
                "Vote2 author mismatch - rejecting potential impersonation attack"
            );
            return Ok(());
        }

        // Verify signature
        if let Err(e) = verify_vote2_signature(&vote, &author, &self.validator_verifier) {
            warn!(
                party_id = %self.party_id,
                vote_author = %vote.author,
                error = ?e,
                "Vote2 signature verification failed"
            );
            return Ok(());
        }

        // Pass to protocol
        match self.protocol.process_vote2(vote).await {
            Ok(Some(qc2)) => {
                info!(
                    party_id = %self.party_id,
                    qc_size = qc2.votes.len(),
                    "QC2 formed, starting Round 3"
                );
                self.start_round3().await?;
            },
            Ok(None) => {
                debug!(
                    party_id = %self.party_id,
                    "Vote2 processed, waiting for more votes"
                );
            },
            Err(e) => {
                warn!(
                    party_id = %self.party_id,
                    error = ?e,
                    "Failed to process Vote2"
                );
            },
        }

        Ok(())
    }

    /// Start Round 3 after QC2 formation
    async fn start_round3(&self) -> Result<()> {
        info!(
            party_id = %self.party_id,
            "Starting Round 3"
        );

        // Protocol creates Vote3 (QC2 is already stored in protocol)
        let (vote3, output) = self.protocol.start_round3(&self.validator_signer).await?;

        info!(
            party_id = %self.party_id,
            vote_author = %vote3.author,
            mcp_prefix_len = vote3.mcp_prefix.len(),
            "Broadcasting Vote3"
        );

        // Broadcast our Vote3
        self.network_sender.broadcast_vote3(vote3).await;

        // If output formed during self-vote processing (early votes accumulated),
        // protocol is already complete — output file will be written on next loop check
        if output.is_some() {
            info!(
                party_id = %self.party_id,
                "QC3 formed during Round 3 start (early votes)"
            );
        }

        Ok(())
    }

    /// Process a Vote3 message
    async fn process_vote3(&self, author: Author, vote: Vote3) -> Result<()> {
        debug!(
            party_id = %self.party_id,
            vote_author = %vote.author,
            mcp_prefix_len = vote.mcp_prefix.len(),
            "Processing Vote3"
        );

        // Check for duplicate
        {
            let mut seen = self.seen_vote3.write().await;
            if seen.contains(&vote.author) {
                debug!(
                    party_id = %self.party_id,
                    vote_author = %vote.author,
                    "Ignoring duplicate Vote3"
                );
                return Ok(());
            }
            seen.insert(vote.author);
        }

        // Check that claimed author matches network sender
        if vote.author != author {
            warn!(
                party_id = %self.party_id,
                claimed_author = %vote.author,
                actual_sender = %author,
                "Vote3 author mismatch - rejecting potential impersonation attack"
            );
            return Ok(());
        }

        // Verify signature
        if let Err(e) = verify_vote3_signature(&vote, &author, &self.validator_verifier) {
            warn!(
                party_id = %self.party_id,
                vote_author = %vote.author,
                error = ?e,
                "Vote3 signature verification failed"
            );
            return Ok(());
        }

        // Pass to protocol
        match self.protocol.process_vote3(vote).await {
            Ok(Some(_qc3)) => {
                info!(
                    party_id = %self.party_id,
                    "QC3 formed, Prefix Consensus complete"
                );

                // Write output to file for smoke test validation
                if let Err(e) = self.write_output_file().await {
                    warn!(
                        party_id = %self.party_id,
                        error = ?e,
                        "Failed to write output file"
                    );
                }
            },
            Ok(None) => {
                debug!(
                    party_id = %self.party_id,
                    "Vote3 processed, waiting for more votes"
                );
            },
            Err(e) => {
                warn!(
                    party_id = %self.party_id,
                    error = ?e,
                    "Failed to process Vote3"
                );
            },
        }

        Ok(())
    }

    /// Check if the protocol has completed
    pub async fn is_complete(&self) -> bool {
        self.protocol.is_complete().await
    }

    /// Get the protocol output if complete
    pub async fn get_output(&self) -> Option<PrefixConsensusOutput> {
        self.protocol.get_output().await
    }

    /// Write output to file for smoke test validation
    async fn write_output_file(&self) -> anyhow::Result<()> {
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize)]
        struct OutputFile {
            party_id: String,
            epoch: u64,
            input: Vec<String>,
            v_low: Vec<String>,
            v_high: Vec<String>,
        }

        let output = self.get_output().await
            .ok_or_else(|| anyhow::anyhow!("Protocol not complete"))?;

        let input_vector = self.protocol.get_input_vector();

        let output_file = OutputFile {
            party_id: format!("{:x}", self.party_id),
            epoch: self.epoch,
            input: input_vector.iter().map(|h| h.to_hex()).collect(),
            v_low: output.v_low.iter().map(|h| h.to_hex()).collect(),
            v_high: output.v_high.iter().map(|h| h.to_hex()).collect(),
        };

        // Write to individual file per validator in /tmp/
        let file_path = format!("/tmp/prefix_consensus_output_{:x}.json", self.party_id);
        let json = serde_json::to_string_pretty(&output_file)?;
        std::fs::write(&file_path, json)?;

        info!(
            party_id = %self.party_id,
            file_path = %file_path,
            "Wrote prefix consensus output file"
        );

        Ok(())
    }

    /// Get the party ID
    pub fn party_id(&self) -> PartyId {
        self.party_id
    }

    /// Get the epoch
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network_interface::PrefixConsensusNetworkSender,
        types::PrefixConsensusInput,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };

    // Mock network sender for testing
    #[derive(Clone)]
    struct MockNetworkSender;

    #[async_trait::async_trait]
    impl PrefixConsensusNetworkSender for MockNetworkSender {
        async fn broadcast_vote1(&self, _vote: Vote1) {}
        async fn broadcast_vote2(&self, _vote: Vote2) {}
        async fn broadcast_vote3(&self, _vote: Vote3) {}
    }

    fn create_test_validators(count: usize) -> (Vec<ValidatorSigner>, Arc<ValidatorVerifier>) {
        let signers: Vec<_> = (0..count)
            .map(|_| ValidatorSigner::random(None))
            .collect();

        let validator_infos: Vec<_> = signers
            .iter()
            .map(|signer| {
                ValidatorConsensusInfo::new(
                    signer.author(),
                    signer.public_key(),
                    1, // voting power
                )
            })
            .collect();

        let verifier = Arc::new(ValidatorVerifier::new(validator_infos));
        (signers, verifier)
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let (mut signers, verifier) = create_test_validators(4);
        let party_id = signers[0].author();

        let input = PrefixConsensusInput::new(
            vec![HashValue::random()], // input_vector
            party_id,                  // party_id
            1,                         // epoch
            0,                         // slot (default for standalone)
            1,                         // view (default for standalone)
        );

        let protocol = Arc::new(PrefixConsensusProtocol::new(input, verifier.clone()));
        let network_sender = MockNetworkSender;

        let manager = PrefixConsensusManager::new(
            party_id,
            1, // epoch
            protocol,
            network_sender,
            signers.remove(0),
            verifier,
        );

        assert_eq!(manager.party_id(), party_id);
        assert_eq!(manager.epoch(), 1);
        assert!(!manager.is_complete().await);
    }

    #[tokio::test]
    async fn test_duplicate_vote_rejection() {
        let (mut signers, verifier) = create_test_validators(4);
        let party_id = signers[0].author();

        let input = PrefixConsensusInput::new(
            vec![HashValue::random()], // input_vector
            party_id,                  // party_id
            1,                         // epoch
            0,                         // slot (default for standalone)
            1,                         // view (default for standalone)
        );

        let protocol = Arc::new(PrefixConsensusProtocol::new(input, verifier.clone()));
        let network_sender = MockNetworkSender;

        let manager = PrefixConsensusManager::new(
            party_id,
            1, // epoch
            protocol,
            network_sender,
            signers.remove(0),
            verifier,
        );

        // Create a duplicate vote (same author twice)
        let vote = Vote1::new(
            signers[1].author(),
            vec![HashValue::random()],
            1,
            0,
            1, // view (default for standalone)
            aptos_crypto::bls12381::Signature::dummy_signature(),
        );

        // First should succeed
        manager.process_vote1(signers[1].author(), vote.clone()).await.unwrap();

        // Second should be ignored (duplicate check)
        let seen_before = manager.seen_vote1.read().await.len();
        manager.process_vote1(signers[1].author(), vote.clone()).await.unwrap();
        let seen_after = manager.seen_vote1.read().await.len();

        assert_eq!(seen_before, seen_after); // Duplicate was ignored
    }

    #[tokio::test]
    async fn test_epoch_mismatch_rejection() {
        let (mut signers, verifier) = create_test_validators(4);
        let party_id = signers[0].author();

        let input = PrefixConsensusInput::new(
            vec![HashValue::random()], // input_vector
            party_id,                  // party_id
            1,                         // epoch 1
            0,                         // slot (default for standalone)
            1,                         // view (default for standalone)
        );

        let protocol = Arc::new(PrefixConsensusProtocol::new(input, verifier.clone()));
        let network_sender = MockNetworkSender;

        let manager = PrefixConsensusManager::new(
            party_id,
            1, // epoch (manager in epoch 1)
            protocol,
            network_sender,
            signers.remove(0),
            verifier,
        );

        // Create vote with wrong epoch
        let vote = Vote1::new(
            signers[1].author(),
            vec![HashValue::random()],
            2, // epoch 2 (wrong!)
            0,
            1, // view (default for standalone)
            aptos_crypto::bls12381::Signature::dummy_signature(),
        );

        let msg = PrefixConsensusMsg::from(vote);

        // Should be rejected due to epoch mismatch
        manager.process_message(signers[1].author(), msg).await.unwrap();

        // Verify vote was not added to seen set
        assert!(manager.seen_vote1.read().await.is_empty());
    }
}
