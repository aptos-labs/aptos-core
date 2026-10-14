// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_consensus_types::{common::Author, pipeline::commit_vote::CommitVote};
use aptos_types::{
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures, SignatureAggregator},
    validator_verifier::ValidatorVerifier,
};
use std::collections::HashMap;

/// Certificate accumulation is independent of execution and local signing progress.
/// Incoming messages have already passed the network verification boundary.
pub(super) struct CommitCertificate {
    ordered_info: BlockInfo,
    early_votes: HashMap<Author, CommitVote>,
    signatures: Option<SignatureAggregator<LedgerInfo>>,
    decision: Option<LedgerInfoWithSignatures>,
}

impl CommitCertificate {
    pub fn new(ordered_info: BlockInfo) -> Self {
        Self {
            ordered_info,
            early_votes: HashMap::new(),
            signatures: None,
            decision: None,
        }
    }

    pub fn add_vote(&mut self, vote: CommitVote) -> bool {
        if let Some(signatures) = &mut self.signatures {
            if signatures.data() != vote.ledger_info() {
                return false;
            }
            signatures.add_signature(vote.author(), vote.signature_with_status());
        } else {
            if !self.ordered_info.match_ordered_only(vote.commit_info()) {
                return false;
            }
            self.early_votes.insert(vote.author(), vote);
        }
        true
    }

    pub fn add_decision(&mut self, decision: LedgerInfoWithSignatures) {
        assert!(self.ordered_info.match_ordered_only(decision.commit_info()));
        self.decision = Some(decision);
    }

    pub fn set_execution_result(&mut self, ledger_info: LedgerInfo) {
        let mut signatures = SignatureAggregator::new(ledger_info);
        for vote in self.early_votes.drain().map(|(_, vote)| vote) {
            // Signatures over another consensus_data_hash must not be combined.
            if vote.ledger_info() == signatures.data() {
                signatures.add_signature(vote.author(), vote.signature_with_status());
            }
        }
        self.signatures = Some(signatures);
    }

    pub fn has_execution_result(&self) -> bool {
        self.signatures.is_some()
    }

    pub fn certificate(
        &mut self,
        verifier: &ValidatorVerifier,
    ) -> Option<LedgerInfoWithSignatures> {
        let signatures = self.signatures.as_mut()?;
        if let Some(decision) = &self.decision {
            // Preserve the fail-stop behavior for a certified execution mismatch.
            assert_eq!(signatures.data().commit_info(), decision.commit_info());
            return Some(decision.clone());
        }
        signatures.check_voting_power(verifier, true).ok()?;
        let _timer = crate::counters::VERIFY_MSG
            .with_label_values(&["commit_vote_aggregate_and_verify"])
            .start_timer();
        signatures
            .aggregate_and_verify(verifier)
            .ok()
            .map(|(ledger_info, signature)| LedgerInfoWithSignatures::new(ledger_info, signature))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::HashValue;
    use aptos_types::{
        ledger_info::generate_ledger_info_with_sig, validator_verifier::random_validator_verifier,
    };

    #[test]
    fn early_votes_require_matching_execution_and_distinct_authors() {
        let (signers, verifier) = random_validator_verifier(4, None, false);
        let li = LedgerInfo::new(BlockInfo::random(1), HashValue::zero());
        let mut certificate = CommitCertificate::new(li.commit_info().clone());
        let vote = CommitVote::new(signers[0].author(), li.clone(), &signers[0]).unwrap();
        for _ in 0..4 {
            assert!(certificate.add_vote(vote.clone()));
        }
        assert!(certificate.certificate(&verifier).is_none());
        certificate.set_execution_result(li.clone());
        assert!(certificate.certificate(&verifier).is_none());
        for signer in &signers[1..3] {
            assert!(
                certificate.add_vote(CommitVote::new(signer.author(), li.clone(), signer).unwrap())
            );
        }
        assert_eq!(
            certificate.certificate(&verifier).unwrap().ledger_info(),
            &li
        );
    }

    #[test]
    fn early_votes_over_another_digest_do_not_count() {
        let (signers, verifier) = random_validator_verifier(4, None, false);
        let li = LedgerInfo::new(BlockInfo::random(1), HashValue::zero());
        let other = LedgerInfo::new(li.commit_info().clone(), HashValue::random());
        let mut certificate = CommitCertificate::new(li.commit_info().clone());
        for signer in &signers {
            assert!(certificate
                .add_vote(CommitVote::new(signer.author(), other.clone(), signer).unwrap()));
        }
        certificate.set_execution_result(li.clone());
        assert!(certificate.certificate(&verifier).is_none());
        assert!(!certificate
            .add_vote(CommitVote::new(signers[0].author(), other, &signers[0]).unwrap()));
    }

    #[test]
    fn decision_before_execution_needs_no_local_vote() {
        let (signers, verifier) = random_validator_verifier(4, None, false);
        let li = LedgerInfo::new(BlockInfo::random(1), HashValue::zero());
        let decision = generate_ledger_info_with_sig(&signers, li.clone());
        let mut certificate = CommitCertificate::new(li.commit_info().clone());
        certificate.add_decision(decision.clone());
        assert!(certificate.certificate(&verifier).is_none());
        certificate.set_execution_result(li);
        assert_eq!(certificate.certificate(&verifier), Some(decision));
    }
}
