// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Prefix Consensus Manager
//!
//! This module provides the event-driven manager that orchestrates the Prefix Consensus
//! protocol lifecycle. It delegates vote processing, signature verification, and round
//! cascading to the [`InnerPCAlgorithm`] trait implementation.

use crate::{
    inner_pc_impl::ThreeRoundPC,
    inner_pc_trait::InnerPCAlgorithm,
    network_interface::PrefixConsensusNetworkSender,
    network_messages::PrefixConsensusMsg,
    types::{PartyId, PrefixConsensusInput, PrefixConsensusOutput, PrefixVector},
};
use anyhow::Result;
use aptos_consensus_types::common::Author;
use aptos_logger::prelude::*;
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use futures::{FutureExt, StreamExt};
use std::sync::Arc;

/// Type alias for the default basic PC Manager using the 3-round protocol.
pub type DefaultPCManager<NS> = PrefixConsensusManager<NS, ThreeRoundPC>;

/// Manager for Prefix Consensus protocol execution
///
/// Orchestrates the protocol lifecycle: receives network messages, delegates
/// vote processing to the inner algorithm, and broadcasts outbound messages.
/// Generic over the inner PC algorithm `T`, allowing different implementations
/// to be swapped in without changing the event loop or output handling logic.
pub struct PrefixConsensusManager<NetworkSender, T: InnerPCAlgorithm> {
    /// This party's ID
    party_id: PartyId,

    /// Current epoch
    epoch: u64,

    /// The inner PC algorithm instance
    algorithm: T,

    /// Network sender for broadcasting votes
    network_sender: NetworkSender,

    /// Validator signer for signing
    validator_signer: ValidatorSigner,

    /// Stored output when protocol completes
    output: Option<PrefixConsensusOutput>,

    /// Input vector (stored for write_output_file)
    input_vector: PrefixVector,
}

impl<NetworkSender: PrefixConsensusNetworkSender, T: InnerPCAlgorithm<Message = PrefixConsensusMsg>>
    PrefixConsensusManager<NetworkSender, T>
{
    /// Create a new Prefix Consensus manager
    pub fn new(
        party_id: PartyId,
        epoch: u64,
        input: PrefixConsensusInput,
        network_sender: NetworkSender,
        validator_signer: ValidatorSigner,
        validator_verifier: Arc<ValidatorVerifier>,
    ) -> Self {
        let input_vector = input.input_vector.clone();
        let algorithm = T::new_for_view(input, validator_verifier);
        Self {
            party_id,
            epoch,
            algorithm,
            network_sender,
            validator_signer,
            output: None,
            input_vector,
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
        mut self,
        mut message_rx: aptos_channels::UnboundedReceiver<(Author, PrefixConsensusMsg)>,
        close_rx: futures::channel::oneshot::Receiver<futures::channel::oneshot::Sender<()>>,
    ) {
        info!(
            party_id = %self.party_id,
            epoch = self.epoch,
            "PrefixConsensusManager event loop started"
        );

        // Start protocol: broadcasts Vote1 and cascades if early QCs form
        match self.algorithm.start(&self.validator_signer).await {
            Ok((msgs, output)) => {
                self.broadcast_messages(msgs).await;
                if let Some(out) = output {
                    self.handle_output(out);
                }
            },
            Err(e) => {
                error!(
                    party_id = %self.party_id,
                    error = ?e,
                    "Failed to start protocol"
                );
                return;
            },
        }

        let mut close_rx = close_rx.into_stream();

        loop {
            // Check if protocol completed during start
            if self.output.is_some() {
                info!(
                    party_id = %self.party_id,
                    "Prefix Consensus protocol complete"
                );
                break;
            }

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
                    if self.output.is_some() {
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
    /// Delegates to the inner algorithm for vote processing, signature verification,
    /// and round cascading. Invalid messages (wrong epoch) are logged and ignored.
    async fn process_message(&mut self, author: Author, msg: PrefixConsensusMsg) -> Result<()> {
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

        match self.algorithm.process_message(author, msg, &self.validator_signer).await {
            Ok((msgs, output)) => {
                self.broadcast_messages(msgs).await;
                if let Some(out) = output {
                    self.handle_output(out);
                }
            },
            Err(e) => {
                warn!(
                    party_id = %self.party_id,
                    error = ?e,
                    "Failed to process message"
                );
            },
        }

        Ok(())
    }

    /// Dispatch returned messages to the correct broadcast method.
    async fn broadcast_messages(&self, messages: Vec<PrefixConsensusMsg>) {
        for msg in messages {
            match msg {
                PrefixConsensusMsg::Vote1Msg(v) => self.network_sender.broadcast_vote1(*v).await,
                PrefixConsensusMsg::Vote2Msg(v) => self.network_sender.broadcast_vote2(*v).await,
                PrefixConsensusMsg::Vote3Msg(v) => self.network_sender.broadcast_vote3(*v).await,
            }
        }
    }

    /// Handle protocol completion output.
    fn handle_output(&mut self, output: PrefixConsensusOutput) {
        info!(
            party_id = %self.party_id,
            v_low_len = output.v_low.len(),
            v_high_len = output.v_high.len(),
            "Prefix Consensus complete"
        );
        self.output = Some(output);
        if let Err(e) = self.write_output_file() {
            warn!(
                party_id = %self.party_id,
                error = ?e,
                "Failed to write output file"
            );
        }
    }

    /// Check if the protocol has completed
    pub fn is_complete(&self) -> bool {
        self.output.is_some()
    }

    /// Get the protocol output if complete
    pub fn get_output(&self) -> Option<&PrefixConsensusOutput> {
        self.output.as_ref()
    }

    /// Write output to file for smoke test validation
    fn write_output_file(&self) -> anyhow::Result<()> {
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize)]
        struct OutputFile {
            party_id: String,
            epoch: u64,
            input: Vec<String>,
            v_low: Vec<String>,
            v_high: Vec<String>,
        }

        let output = self.output.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Protocol not complete"))?;

        let output_file = OutputFile {
            party_id: format!("{:x}", self.party_id),
            epoch: self.epoch,
            input: self.input_vector.iter().map(|h| h.to_hex()).collect(),
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
        types::{PrefixConsensusInput, Vote1, Vote2, Vote3},
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

        let network_sender = MockNetworkSender;

        let manager = DefaultPCManager::new(
            party_id,
            1, // epoch
            input,
            network_sender,
            signers.remove(0),
            verifier,
        );

        assert_eq!(manager.party_id(), party_id);
        assert_eq!(manager.epoch(), 1);
        assert!(!manager.is_complete());
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

        let network_sender = MockNetworkSender;

        let mut manager = DefaultPCManager::new(
            party_id,
            1, // epoch (manager in epoch 1)
            input,
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

        // Verify protocol did not complete
        assert!(!manager.is_complete());
    }
}
