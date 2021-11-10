// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::persistent_liveness_storage::PersistentLivenessStorage;
use consensus_types::{
    block_data::BlockData,
    timeout::Timeout,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
    vote::Vote,
    vote_proposal::MaybeSignedVoteProposal,
};
use diem_crypto::ed25519::Ed25519Signature;
use diem_logger::prelude::info;
use diem_metrics::monitor;
use diem_types::{
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
use safety_rules::{ConsensusState, Error, TSafetyRules};
use std::sync::Arc;

/// Wrap safety rules with counters.
pub struct MetricsSafetyRules {
    inner: Box<dyn TSafetyRules + Send + Sync>,
    storage: Arc<dyn PersistentLivenessStorage>,
}

impl MetricsSafetyRules {
    pub fn new(
        inner: Box<dyn TSafetyRules + Send + Sync>,
        storage: Arc<dyn PersistentLivenessStorage>,
    ) -> Self {
        Self { inner, storage }
    }

    pub fn perform_initialize(&mut self) -> Result<(), Error> {
        let consensus_state = self.consensus_state()?;
        let mut waypoint_version = consensus_state.waypoint().version();
        loop {
            let proofs = self
                .storage
                .retrieve_epoch_change_proof(waypoint_version)
                .map_err(|e| {
                    Error::InternalError(format!(
                        "Unable to retrieve Waypoint state from storage, encountered Error:{}",
                        e
                    ))
                })?;
            // We keep initializing safety rules as long as the waypoint continues to increase.
            // This is due to limits in the number of epoch change proofs that storage can provide.
            match self.initialize(&proofs) {
                Err(Error::WaypointOutOfDate(
                    prev_version,
                    curr_version,
                    current_epoch,
                    provided_epoch,
                )) if prev_version < curr_version => {
                    waypoint_version = curr_version;
                    info!("Previous waypoint version {}, updated version {}, current epoch {}, provided epoch {}", prev_version, curr_version, current_epoch, provided_epoch);
                    continue;
                }
                result => return result,
            }
        }
    }

    fn retry<T, F: FnMut(&mut Box<dyn TSafetyRules + Send + Sync>) -> Result<T, Error>>(
        &mut self,
        mut f: F,
    ) -> Result<T, Error> {
        let result = f(&mut self.inner);
        match result {
            Err(Error::NotInitialized(_))
            | Err(Error::IncorrectEpoch(_, _))
            | Err(Error::WaypointOutOfDate(_, _, _, _)) => {
                self.perform_initialize()?;
                f(&mut self.inner)
            }
            _ => result,
        }
    }
}

impl TSafetyRules for MetricsSafetyRules {
    fn consensus_state(&mut self) -> Result<ConsensusState, Error> {
        monitor!("safety_rules", self.inner.consensus_state())
    }

    fn initialize(&mut self, proof: &EpochChangeProof) -> Result<(), Error> {
        monitor!("safety_rules", self.inner.initialize(proof))
    }

    fn construct_and_sign_vote(
        &mut self,
        vote_proposal: &MaybeSignedVoteProposal,
    ) -> Result<Vote, Error> {
        self.retry(|inner| monitor!("safety_rules", inner.construct_and_sign_vote(vote_proposal)))
    }

    fn sign_proposal(&mut self, block_data: &BlockData) -> Result<Ed25519Signature, Error> {
        self.retry(|inner| monitor!("safety_rules", inner.sign_proposal(block_data)))
    }

    fn sign_timeout(&mut self, timeout: &Timeout) -> Result<Ed25519Signature, Error> {
        self.retry(|inner| monitor!("safety_rules", inner.sign_timeout(timeout)))
    }

    fn sign_timeout_with_qc(
        &mut self,
        timeout: &TwoChainTimeout,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<Ed25519Signature, Error> {
        self.retry(|inner| {
            monitor!(
                "safety_rules",
                inner.sign_timeout_with_qc(timeout, timeout_cert)
            )
        })
    }

    fn construct_and_sign_vote_two_chain(
        &mut self,
        vote_proposal: &MaybeSignedVoteProposal,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<Vote, Error> {
        self.retry(|inner| {
            monitor!(
                "safety_rules",
                inner.construct_and_sign_vote_two_chain(vote_proposal, timeout_cert)
            )
        })
    }

    fn sign_commit_vote(
        &mut self,
        ledger_info: LedgerInfoWithSignatures,
        new_ledger_info: LedgerInfo,
    ) -> Result<Ed25519Signature, Error> {
        self.retry(|inner| {
            monitor!(
                "safety_rules",
                inner.sign_commit_vote(ledger_info.clone(), new_ledger_info.clone())
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{metrics_safety_rules::MetricsSafetyRules, test_utils::EmptyStorage};
    use claim::{assert_matches, assert_ok};
    use consensus_types::{
        block_data::BlockData,
        timeout::Timeout,
        timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
        vote::Vote,
        vote_proposal::MaybeSignedVoteProposal,
    };
    use diem_crypto::ed25519::Ed25519Signature;
    use diem_types::{
        epoch_change::EpochChangeProof,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    };
    use safety_rules::{ConsensusState, Error, TSafetyRules};

    pub struct MockSafetyRules {
        // number of initialize() calls
        init_calls: i32,

        // max initialize() calls to complete perform_initialize()
        max_init_calls: i32,

        // last initialize() returns Ok() or any error != WaypointOutOfDate
        last_init_result: Result<(), Error>,
    }

    impl MockSafetyRules {
        pub fn new(
            init_calls: i32,
            max_init_calls: i32,
            last_init_result: Result<(), Error>,
        ) -> Self {
            Self {
                init_calls,
                max_init_calls,
                last_init_result,
            }
        }
    }

    impl TSafetyRules for MockSafetyRules {
        fn consensus_state(&mut self) -> Result<ConsensusState, Error> {
            Ok(ConsensusState::default())
        }

        fn initialize(&mut self, _: &EpochChangeProof) -> Result<(), Error> {
            self.init_calls += 1;
            if self.init_calls < self.max_init_calls {
                return Err(Error::WaypointOutOfDate(
                    (self.init_calls - 1) as u64,
                    self.init_calls as u64,
                    self.max_init_calls as u64,
                    self.init_calls as u64,
                ));
            }
            self.last_init_result.clone()
        }

        fn construct_and_sign_vote(&mut self, _: &MaybeSignedVoteProposal) -> Result<Vote, Error> {
            unimplemented!()
        }

        fn sign_proposal(&mut self, _: &BlockData) -> Result<Ed25519Signature, Error> {
            unimplemented!()
        }

        fn sign_timeout(&mut self, _: &Timeout) -> Result<Ed25519Signature, Error> {
            unimplemented!()
        }

        fn sign_timeout_with_qc(
            &mut self,
            _: &TwoChainTimeout,
            _: Option<&TwoChainTimeoutCertificate>,
        ) -> Result<Ed25519Signature, Error> {
            unimplemented!()
        }

        fn construct_and_sign_vote_two_chain(
            &mut self,
            _: &MaybeSignedVoteProposal,
            _: Option<&TwoChainTimeoutCertificate>,
        ) -> Result<Vote, Error> {
            unimplemented!()
        }

        fn sign_commit_vote(
            &mut self,
            _: LedgerInfoWithSignatures,
            _: LedgerInfo,
        ) -> Result<Ed25519Signature, Error> {
            unimplemented!()
        }
    }

    #[test]
    fn test_perform_initialize_ok() {
        ::diem_logger::Logger::init_for_testing();
        let (_, mock_storage) = EmptyStorage::start_for_testing();
        let mock_safety_rules = MockSafetyRules::new(0, 10, Ok(()));
        let mut metric_safety_rules =
            MetricsSafetyRules::new(Box::new(mock_safety_rules), mock_storage);
        assert_ok!(metric_safety_rules.perform_initialize());
    }

    #[test]
    fn test_perform_initialize_error() {
        ::diem_logger::Logger::init_for_testing();
        let (_, mock_storage) = EmptyStorage::start_for_testing();
        let mock_safety_rules = MockSafetyRules::new(
            0,
            10,
            Err(Error::InvalidEpochChangeProof(String::from("Error"))),
        );
        let mut metric_safety_rules =
            MetricsSafetyRules::new(Box::new(mock_safety_rules), mock_storage);
        assert_matches!(
            metric_safety_rules.perform_initialize(),
            Err(Error::InvalidEpochChangeProof(_))
        );
    }
}
