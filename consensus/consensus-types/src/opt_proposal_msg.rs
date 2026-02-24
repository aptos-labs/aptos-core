// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::Author, opt_block_data::OptBlockData, proof_of_store::ProofCache, sync_info::SyncInfo,
};
use anyhow::{ensure, Context, Result};
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OptProposalMsg {
    block_data: OptBlockData,
    sync_info: SyncInfo,
}

impl OptProposalMsg {
    pub fn new(block_data: OptBlockData, sync_info: SyncInfo) -> Self {
        Self {
            block_data,
            sync_info,
        }
    }

    pub fn block_data(&self) -> &OptBlockData {
        &self.block_data
    }

    pub fn take_block_data(self) -> OptBlockData {
        self.block_data
    }

    pub fn epoch(&self) -> u64 {
        self.block_data.epoch()
    }

    pub fn round(&self) -> u64 {
        self.block_data.round()
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.block_data.timestamp_usecs()
    }

    pub fn proposer(&self) -> Author {
        *self.block_data.author()
    }

    pub fn sync_info(&self) -> &SyncInfo {
        &self.sync_info
    }

    /// Verifies that the ProposalMsg is well-formed.
    pub fn verify_well_formed(&self) -> Result<()> {
        self.block_data
            .verify_well_formed()
            .context("Fail to verify OptProposalMsg's data")?;
        ensure!(
            self.block_data.round() > 1,
            "Proposal for {} has round <= 1",
            self.block_data,
        );
        ensure!(
            self.block_data.epoch() == self.sync_info.epoch(),
            "ProposalMsg has different epoch number from SyncInfo"
        );
        // Ensure the sync info has the grandparent QC
        ensure!(
            self.block_data.grandparent_qc().certified_block().id()
                == self.sync_info.highest_quorum_cert().certified_block().id(),
            "Proposal HQC in SyncInfo certifies {}, but block grandparent id is {}",
            self.sync_info.highest_quorum_cert().certified_block().id(),
            self.block_data.grandparent_qc().certified_block().id(),
        );
        let grandparent_round = self
            .block_data
            .round()
            .checked_sub(2)
            .ok_or_else(|| anyhow::anyhow!("proposal round overflowed!"))?;

        let highest_certified_round = self.block_data.grandparent_qc().certified_block().round();
        ensure!(
            grandparent_round == highest_certified_round,
            "Proposal {} does not have a certified round {}",
            self.block_data,
            grandparent_round
        );
        // Optimistic proposal shouldn't have a timeout certificate
        ensure!(
            self.sync_info.highest_2chain_timeout_cert().is_none(),
            "Optimistic proposal shouldn't have a timeout certificate"
        );
        Ok(())
    }

    pub fn verify(
        &self,
        sender: Author,
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
        quorum_store_enabled: bool,
        opt_qs_v2_enabled: bool,
    ) -> Result<()> {
        ensure!(
            self.proposer() == sender,
            "OptProposal author {:?} doesn't match sender {:?}",
            self.proposer(),
            sender
        );

        let (payload_verify_result, qc_verify_result) = rayon::join(
            || {
                self.block_data().payload().verify(
                    validator,
                    proof_cache,
                    quorum_store_enabled,
                    opt_qs_v2_enabled,
                )
            },
            || self.block_data().grandparent_qc().verify(validator),
        );
        payload_verify_result?;
        qc_verify_result?;

        // Note that we postpone the verification of SyncInfo until it's being used.
        self.verify_well_formed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block::block_test_utils::{certificate_for_genesis, gen_test_certificate},
        common::{Payload, Round},
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        block_info::BlockInfo, validator_signer::ValidatorSigner,
        validator_verifier::random_validator_verifier,
    };

    // Helper to create a OptProposalMsg for testing
    fn create_opt_proposal_msg(
        round: Round,
        epoch: u64,
        signer: &ValidatorSigner,
    ) -> OptProposalMsg {
        let grandparent_round = round.saturating_sub(2);
        let grandparent_block = BlockInfo::new(
            epoch,
            grandparent_round,
            HashValue::zero(),
            HashValue::zero(),
            0,
            grandparent_round * 1000,
            None,
        );

        // Create a parent block info for the QC (the block before grandparent)
        let parent_of_grandparent = BlockInfo::new(
            epoch,
            grandparent_round.saturating_sub(1),
            HashValue::zero(),
            HashValue::zero(),
            0,
            grandparent_round.saturating_sub(1) * 1000,
            None,
        );

        let parent_round = round.saturating_sub(1);
        // Parent block's id should be the grandparent block's id
        let parent_block = BlockInfo::new(
            epoch,
            parent_round,
            grandparent_block.id(), // Parent points to grandparent
            HashValue::zero(),
            0,
            parent_round * 1000,
            None,
        );

        let grandparent_qc = gen_test_certificate(
            std::slice::from_ref(signer),
            grandparent_block.clone(),
            parent_of_grandparent, // Use proper parent instead of BlockInfo::empty()
            None,
        );

        let opt_block_data = OptBlockData::new(
            vec![],
            Payload::empty(false),
            signer.author(),
            epoch,
            round,
            round * 1000,
            parent_block,
            grandparent_qc.clone(),
        );

        let sync_info = SyncInfo::new(
            grandparent_qc.clone(),
            grandparent_qc.into_wrapped_ledger_info(),
            None,
        );

        OptProposalMsg::new(opt_block_data, sync_info)
    }

    // ========== verify() Tests ==========

    #[test]
    fn test_verify_success() {
        let (signers, validators) = random_validator_verifier(1, None, false);
        let signer = signers.first().unwrap();
        let msg = create_opt_proposal_msg(3, 1, signer);
        let proof_cache = ProofCache::new(1024);
        assert!(msg
            .verify(signer.author(), &validators, &proof_cache, false, false)
            .is_ok());
    }

    #[test]
    fn test_verify_failures() {
        let (signers, validators) = random_validator_verifier(1, None, false);
        let signer = signers.first().unwrap();
        let proof_cache = ProofCache::new(1024);

        // Test round too low
        let msg_round_1 = create_opt_proposal_msg(1, 1, signer);
        assert!(msg_round_1
            .verify(signer.author(), &validators, &proof_cache, false, false)
            .is_err());

        // Test epoch mismatch
        let msg = create_opt_proposal_msg(3, 1, signer);
        let genesis_qc = certificate_for_genesis();
        let sync_info = SyncInfo::new(
            genesis_qc.clone(),
            genesis_qc.into_wrapped_ledger_info(),
            None,
        );
        let block_data = msg.take_block_data();
        let epoch_2_block_data = OptBlockData::new(
            vec![],
            Payload::empty(false),
            signer.author(),
            2, // Different epoch
            block_data.round(),
            block_data.timestamp_usecs(),
            block_data.parent().clone(),
            block_data.grandparent_qc().clone(),
        );
        let msg_epoch_mismatch = OptProposalMsg::new(epoch_2_block_data, sync_info);
        assert!(msg_epoch_mismatch
            .verify(signer.author(), &validators, &proof_cache, false, false)
            .is_err());

        // Test with timeout cert
        let msg = create_opt_proposal_msg(3, 1, signer);
        let block_data = msg.take_block_data();
        let grandparent_qc = block_data.grandparent_qc().clone();
        use crate::timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate};
        let timeout = TwoChainTimeout::new(1, 2, grandparent_qc.clone());
        let timeout_cert = TwoChainTimeoutCertificate::new(timeout);
        let sync_info = SyncInfo::new(
            grandparent_qc.clone(),
            grandparent_qc.into_wrapped_ledger_info(),
            Some(timeout_cert),
        );
        let msg_with_tc = OptProposalMsg::new(block_data, sync_info);
        assert!(msg_with_tc
            .verify(signer.author(), &validators, &proof_cache, false, false)
            .is_err());
    }

    #[test]
    fn test_verify_sender_mismatch() {
        let (signers, validators) = random_validator_verifier(2, None, false);
        let signer1 = &signers[0];
        let signer2 = &signers[1];

        let msg = create_opt_proposal_msg(3, 1, signer1);
        let proof_cache = ProofCache::new(1024);

        assert!(msg
            .verify(signer2.author(), &validators, &proof_cache, false, false)
            .is_err());
    }
}
