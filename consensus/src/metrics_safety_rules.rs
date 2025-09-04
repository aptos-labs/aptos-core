// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor, persistent_liveness_storage::PersistentLivenessStorage,
    pipeline::signing_phase::CommitSignerProvider,
};
use velor_consensus_types::{
    block_data::BlockData,
    order_vote::OrderVote,
    order_vote_proposal::OrderVoteProposal,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
    vote::Vote,
    vote_proposal::VoteProposal,
};
use velor_crypto::bls12381;
use velor_infallible::Mutex;
use velor_logger::prelude::info;
use velor_safety_rules::{ConsensusState, Error, TSafetyRules};
use velor_types::{
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
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
                },
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
            },
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

    fn sign_proposal(&mut self, block_data: &BlockData) -> Result<bls12381::Signature, Error> {
        self.retry(|inner| monitor!("safety_rules", inner.sign_proposal(block_data)))
    }

    fn sign_timeout_with_qc(
        &mut self,
        timeout: &TwoChainTimeout,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<bls12381::Signature, Error> {
        self.retry(|inner| {
            monitor!(
                "safety_rules",
                inner.sign_timeout_with_qc(timeout, timeout_cert)
            )
        })
    }

    fn construct_and_sign_vote_two_chain(
        &mut self,
        vote_proposal: &VoteProposal,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<Vote, Error> {
        self.retry(|inner| {
            monitor!(
                "safety_rules",
                inner.construct_and_sign_vote_two_chain(vote_proposal, timeout_cert)
            )
        })
    }

    fn construct_and_sign_order_vote(
        &mut self,
        order_vote_proposal: &OrderVoteProposal,
    ) -> Result<OrderVote, Error> {
        self.retry(|inner| {
            monitor!(
                "safety_rules",
                inner.construct_and_sign_order_vote(order_vote_proposal)
            )
        })
    }

    fn sign_commit_vote(
        &mut self,
        ledger_info: LedgerInfoWithSignatures,
        new_ledger_info: LedgerInfo,
    ) -> Result<bls12381::Signature, Error> {
        self.retry(|inner| {
            monitor!(
                "safety_rules",
                inner.sign_commit_vote(ledger_info.clone(), new_ledger_info.clone())
            )
        })
    }
}

impl CommitSignerProvider for Mutex<MetricsSafetyRules> {
    fn sign_commit_vote(
        &self,
        ledger_info: LedgerInfoWithSignatures,
        new_ledger_info: LedgerInfo,
    ) -> Result<bls12381::Signature, Error> {
        self.lock().sign_commit_vote(ledger_info, new_ledger_info)
    }
}

#[cfg(test)]
mod tests {
    use crate::{metrics_safety_rules::MetricsSafetyRules, test_utils::EmptyStorage};
    use velor_consensus_types::{
        block_data::BlockData,
        order_vote::OrderVote,
        order_vote_proposal::OrderVoteProposal,
        timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
        vote::Vote,
        vote_proposal::VoteProposal,
    };
    use velor_crypto::bls12381;
    use velor_safety_rules::{ConsensusState, Error, TSafetyRules};
    use velor_types::{
        epoch_change::EpochChangeProof,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    };
    use claims::{assert_matches, assert_ok};

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

        fn sign_proposal(&mut self, _: &BlockData) -> Result<bls12381::Signature, Error> {
            unimplemented!()
        }

        fn sign_timeout_with_qc(
            &mut self,
            _: &TwoChainTimeout,
            _: Option<&TwoChainTimeoutCertificate>,
        ) -> Result<bls12381::Signature, Error> {
            unimplemented!()
        }

        fn construct_and_sign_vote_two_chain(
            &mut self,
            _: &VoteProposal,
            _: Option<&TwoChainTimeoutCertificate>,
        ) -> Result<Vote, Error> {
            unimplemented!()
        }

        fn construct_and_sign_order_vote(
            &mut self,
            _: &OrderVoteProposal,
        ) -> Result<OrderVote, Error> {
            unimplemented!()
        }

        fn sign_commit_vote(
            &mut self,
            _: LedgerInfoWithSignatures,
            _: LedgerInfo,
        ) -> Result<bls12381::Signature, Error> {
            unimplemented!()
        }
    }

    #[test]
    fn test_perform_initialize_ok() {
        ::velor_logger::Logger::init_for_testing();
        let (_, mock_storage) = EmptyStorage::start_for_testing();
        let mock_safety_rules = MockSafetyRules::new(0, 10, Ok(()));
        let mut metric_safety_rules =
            MetricsSafetyRules::new(Box::new(mock_safety_rules), mock_storage);
        assert_ok!(metric_safety_rules.perform_initialize());
    }

    #[test]
    fn test_perform_initialize_error() {
        ::velor_logger::Logger::init_for_testing();
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
