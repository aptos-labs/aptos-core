// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implementation of [`InnerPCAlgorithm`] for the standard 3-round protocol.
//!
//! [`ThreeRoundPC`] wraps [`PrefixConsensusProtocol`] and handles the 3-round
//! state machine (Vote1→QC1→Vote2→QC2→Vote3→QC3→Output) internally,
//! including author mismatch checks and signature verification.

use crate::{
    inner_pc_trait::{Author, InnerPCAlgorithm},
    network_messages::PrefixConsensusMsg,
    protocol::PrefixConsensusProtocol,
    signing::{verify_vote1_signature, verify_vote2_signature, verify_vote3_signature},
    types::{PrefixConsensusInput, PrefixConsensusOutput},
};
use anyhow::Result;
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use async_trait::async_trait;
use std::sync::Arc;
use aptos_logger::prelude::*;

/// Tracks which round the cascade logic should advance to next.
///
/// This is separate from `ProtocolState` inside `PrefixConsensusProtocol`:
/// - `RoundState` tracks "what to start next on QC formation" (cascade dispatch)
/// - `ProtocolState` guards internal state transitions (invariant enforcement)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoundState {
    Round1,
    Round2,
    Round3,
    Complete,
}

/// Implementation of [`InnerPCAlgorithm`] using the standard 3-round protocol.
///
/// Wraps [`PrefixConsensusProtocol`] and handles the full 3-round state machine
/// (Vote1→QC1→Vote2→QC2→Vote3→QC3→Output) internally. The manager only sees
/// `start()` and `process_message()`.
pub struct ThreeRoundPC {
    protocol: PrefixConsensusProtocol,
    verifier: Arc<ValidatorVerifier>,
    round: RoundState,
}

impl ThreeRoundPC {
    /// Cascade from Round 1 completion: start Round 2, and if QC2 forms
    /// immediately, cascade to Round 3.
    ///
    /// Returns outbound messages and optional completion output.
    async fn cascade_from_round2(
        &mut self,
        signer: &ValidatorSigner,
    ) -> Result<(Vec<PrefixConsensusMsg>, Option<PrefixConsensusOutput>)> {
        let (vote2, qc2) = self.protocol.start_round2(signer).await?;
        self.round = RoundState::Round2;

        let mut msgs = vec![PrefixConsensusMsg::from(vote2)];

        if qc2.is_some() {
            let (more_msgs, output) = self.cascade_from_round3(signer).await?;
            msgs.extend(more_msgs);
            return Ok((msgs, output));
        }

        Ok((msgs, None))
    }

    /// Cascade from Round 2 completion: start Round 3.
    ///
    /// Returns outbound messages and optional completion output.
    async fn cascade_from_round3(
        &mut self,
        signer: &ValidatorSigner,
    ) -> Result<(Vec<PrefixConsensusMsg>, Option<PrefixConsensusOutput>)> {
        let (vote3, output) = self.protocol.start_round3(signer).await?;
        self.round = if output.is_some() {
            RoundState::Complete
        } else {
            RoundState::Round3
        };

        Ok((vec![PrefixConsensusMsg::from(vote3)], output))
    }
}

#[async_trait]
impl InnerPCAlgorithm for ThreeRoundPC {
    type Message = PrefixConsensusMsg;

    fn new_for_view(input: PrefixConsensusInput, verifier: Arc<ValidatorVerifier>) -> Self {
        let protocol = PrefixConsensusProtocol::new(input, verifier.clone());
        Self {
            protocol,
            verifier,
            round: RoundState::Round1,
        }
    }

    async fn start(
        &mut self,
        signer: &ValidatorSigner,
    ) -> Result<(Vec<Self::Message>, Option<PrefixConsensusOutput>)> {
        let (vote1, qc1) = self.protocol.start_round1(signer).await?;

        let mut msgs = vec![PrefixConsensusMsg::from(vote1)];

        if qc1.is_some() {
            // Early QC1 — cascade to Round 2 (and potentially Round 3)
            let (more_msgs, output) = self.cascade_from_round2(signer).await?;
            msgs.extend(more_msgs);
            return Ok((msgs, output));
        }

        Ok((msgs, None))
    }

    async fn process_message(
        &mut self,
        author: Author,
        msg: Self::Message,
        signer: &ValidatorSigner,
    ) -> Result<(Vec<Self::Message>, Option<PrefixConsensusOutput>)> {
        match msg {
            PrefixConsensusMsg::Vote1Msg(vote) => {
                // Author mismatch check (security: network-authenticated sender)
                if vote.author != author {
                    return Ok((vec![], None));
                }
                // Signature verification
                if verify_vote1_signature(&vote, &author, &self.verifier).is_err() {
                    return Ok((vec![], None));
                }
                // Process vote
                match self.protocol.process_vote1(*vote).await {
                    Ok(Some(_qc1)) => {
                        // QC1 formed — cascade to Round 2
                        self.cascade_from_round2(signer).await
                    },
                    Ok(None) => Ok((vec![], None)),
                    Err(e) => {
                        warn!(error = ?e, "ThreeRoundPC: process_vote1 error");
                        Err(e)
                    },
                }
            },
            PrefixConsensusMsg::Vote2Msg(vote) => {
                if vote.author != author {
                    return Ok((vec![], None));
                }
                if verify_vote2_signature(&vote, &author, &self.verifier).is_err() {
                    return Ok((vec![], None));
                }
                match self.protocol.process_vote2(*vote).await {
                    Ok(Some(_qc2)) => {
                        // QC2 formed — cascade to Round 3
                        self.cascade_from_round3(signer).await
                    },
                    Ok(None) => Ok((vec![], None)),
                    Err(e) => {
                        warn!(error = ?e, "ThreeRoundPC: process_vote2 error");
                        Err(e)
                    },
                }
            },
            PrefixConsensusMsg::Vote3Msg(vote) => {
                if vote.author != author {
                    return Ok((vec![], None));
                }
                if verify_vote3_signature(&vote, &author, &self.verifier).is_err() {
                    return Ok((vec![], None));
                }
                match self.protocol.process_vote3(*vote).await {
                    Ok(Some(output)) => {
                        self.round = RoundState::Complete;
                        Ok((vec![], Some(output)))
                    },
                    Ok(None) => Ok((vec![], None)),
                    Err(e) => {
                        warn!(error = ?e, "ThreeRoundPC: process_vote3 error");
                        Err(e)
                    },
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::HashValue;
    use aptos_types::validator_verifier::random_validator_verifier;

    /// Helper: create N validators with signers and a ValidatorVerifier
    fn setup_validators(n: usize) -> (Vec<ValidatorSigner>, Arc<ValidatorVerifier>) {
        let (signers, verifier) = random_validator_verifier(n, None, false);
        (signers, Arc::new(verifier))
    }

    /// Helper: create a PrefixConsensusInput for a given signer
    fn make_input(
        signer: &ValidatorSigner,
        hashes: Vec<HashValue>,
        epoch: u64,
        view: u64,
    ) -> PrefixConsensusInput {
        PrefixConsensusInput::new(hashes, signer.author(), epoch, 0, view)
    }

    /// Helper: generate deterministic test hashes
    fn test_hashes(count: usize) -> Vec<HashValue> {
        (1..=count)
            .map(|i| HashValue::sha3_256_of(&(i as u64).to_le_bytes()))
            .collect()
    }

    #[tokio::test]
    async fn test_start_returns_vote1() {
        let (signers, verifier) = setup_validators(4);
        let input = make_input(&signers[0], test_hashes(3), 1, 1);

        let mut pc = ThreeRoundPC::new_for_view(input, verifier);
        let (msgs, output) = pc.start(&signers[0]).await.unwrap();

        // Should return at least one message (Vote1)
        assert!(!msgs.is_empty());
        assert!(
            matches!(&msgs[0], PrefixConsensusMsg::Vote1Msg(_)),
            "First message should be Vote1"
        );
        // With 4 validators and only 1 self-vote, no early QC → no output
        assert!(output.is_none());
    }

    #[tokio::test]
    async fn test_process_vote1_qc_cascade() {
        let (signers, verifier) = setup_validators(4);
        let hashes = test_hashes(3);

        // Create 4 instances with identical inputs
        let mut instances: Vec<ThreeRoundPC> = signers
            .iter()
            .map(|s| {
                let input = make_input(s, hashes.clone(), 1, 1);
                ThreeRoundPC::new_for_view(input, verifier.clone())
            })
            .collect();

        // Start all instances — collect Vote1 messages
        let mut vote1_msgs: Vec<(Author, PrefixConsensusMsg)> = Vec::new();
        for (i, inst) in instances.iter_mut().enumerate() {
            let (msgs, _) = inst.start(&signers[i]).await.unwrap();
            for msg in msgs {
                if matches!(&msg, PrefixConsensusMsg::Vote1Msg(_)) {
                    vote1_msgs.push((signers[i].author(), msg));
                }
            }
        }

        // Feed all Vote1 messages to instance 0 — should trigger QC1 cascade
        let mut got_vote2 = false;
        for (author, msg) in &vote1_msgs {
            if *author == signers[0].author() {
                continue; // Skip self (already processed in start)
            }
            let (out_msgs, _output) = instances[0]
                .process_message(*author, msg.clone(), &signers[0])
                .await
                .unwrap();

            for m in &out_msgs {
                if matches!(m, PrefixConsensusMsg::Vote2Msg(_)) {
                    got_vote2 = true;
                }
            }
        }

        assert!(got_vote2, "QC1 should form and cascade to Vote2");
    }

    #[tokio::test]
    async fn test_full_protocol_through_trait() {
        let (signers, verifier) = setup_validators(4);
        let hashes = test_hashes(3);
        let n = signers.len();

        // Create 4 instances with identical inputs
        let mut instances: Vec<ThreeRoundPC> = signers
            .iter()
            .map(|s| {
                let input = make_input(s, hashes.clone(), 1, 1);
                ThreeRoundPC::new_for_view(input, verifier.clone())
            })
            .collect();

        // Start all — collect all outbound messages
        let mut pending: Vec<(Author, PrefixConsensusMsg)> = Vec::new();
        for (i, inst) in instances.iter_mut().enumerate() {
            let (msgs, _) = inst.start(&signers[i]).await.unwrap();
            for msg in msgs {
                pending.push((signers[i].author(), msg));
            }
        }

        // Drive protocol: feed pending messages to all instances, collect new outbound
        let mut outputs: Vec<Option<PrefixConsensusOutput>> = vec![None; n];
        let mut iterations = 0;

        while !pending.is_empty() && iterations < 100 {
            iterations += 1;
            let batch = std::mem::take(&mut pending);

            for (author, msg) in batch {
                for i in 0..n {
                    if outputs[i].is_some() {
                        continue; // Already completed
                    }
                    let (out_msgs, output) = instances[i]
                        .process_message(author, msg.clone(), &signers[i])
                        .await
                        .unwrap();

                    for m in out_msgs {
                        pending.push((signers[i].author(), m));
                    }

                    if let Some(out) = output {
                        outputs[i] = Some(out);
                    }
                }
            }
        }

        // All 4 should have completed
        for (i, output) in outputs.iter().enumerate() {
            assert!(
                output.is_some(),
                "Instance {} did not complete after {} iterations",
                i, iterations
            );
        }

        // All outputs should have identical v_low and v_high
        let first = outputs[0].as_ref().unwrap();
        for (i, output) in outputs.iter().enumerate().skip(1) {
            let out = output.as_ref().unwrap();
            assert_eq!(
                first.v_low, out.v_low,
                "Instance {} v_low differs from instance 0",
                i
            );
            assert_eq!(
                first.v_high, out.v_high,
                "Instance {} v_high differs from instance 0",
                i
            );
        }

        // For identical inputs, v_low and v_high should equal the input
        assert_eq!(first.v_low, hashes);
        assert_eq!(first.v_high, hashes);
    }

    #[tokio::test]
    async fn test_author_mismatch_rejected() {
        let (signers, verifier) = setup_validators(4);
        let hashes = test_hashes(3);

        let mut inst0 = ThreeRoundPC::new_for_view(
            make_input(&signers[0], hashes.clone(), 1, 1),
            verifier.clone(),
        );
        let mut inst1 = ThreeRoundPC::new_for_view(
            make_input(&signers[1], hashes.clone(), 1, 1),
            verifier.clone(),
        );

        // Start both
        inst0.start(&signers[0]).await.unwrap();
        let (msgs1, _) = inst1.start(&signers[1]).await.unwrap();

        // Take inst1's Vote1 but claim it's from inst2 (author mismatch)
        let vote1 = msgs1
            .into_iter()
            .find(|m| matches!(m, PrefixConsensusMsg::Vote1Msg(_)))
            .unwrap();

        // Send with wrong author (signers[2] instead of signers[1])
        let (out_msgs, output) = inst0
            .process_message(signers[2].author(), vote1, &signers[0])
            .await
            .unwrap();

        // Should be silently dropped
        assert!(out_msgs.is_empty(), "Mismatched author should be dropped");
        assert!(output.is_none(), "Mismatched author should produce no output");
    }
}
