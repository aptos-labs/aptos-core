// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::vote_data::VoteData;
use anyhow::{ensure, Context};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// This struct is similar to QuorumCert, except that the verify function doesn't verify vote_data.
/// vote_data and consensus_data_hash inside signed_ledger_info could be replaced with dummy values.
#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
pub struct WrappedLedgerInfo {
    /// The vote information certified by the quorum.
    vote_data: VoteData,
    /// The signed LedgerInfo of a committed block that carries the data about the certified block.
    signed_ledger_info: LedgerInfoWithSignatures,
}

impl Display for WrappedLedgerInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "WrappedLedgerInfo: [{}, {}]",
            self.vote_data, self.signed_ledger_info
        )
    }
}

impl WrappedLedgerInfo {
    pub fn new(vote_data: VoteData, signed_ledger_info: LedgerInfoWithSignatures) -> Self {
        WrappedLedgerInfo {
            vote_data,
            signed_ledger_info,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self {
            vote_data: VoteData::dummy(),
            signed_ledger_info: LedgerInfoWithSignatures::new(
                LedgerInfo::dummy(),
                AggregateSignature::empty(),
            ),
        }
    }

    pub fn vote_data(&self) -> &VoteData {
        &self.vote_data
    }

    pub fn certified_block(&self) -> &BlockInfo {
        self.vote_data.proposed()
    }

    pub fn parent_block(&self) -> &BlockInfo {
        self.vote_data.parent()
    }

    pub fn ledger_info(&self) -> &LedgerInfoWithSignatures {
        &self.signed_ledger_info
    }

    pub fn commit_info(&self) -> &BlockInfo {
        self.ledger_info().ledger_info().commit_info()
    }

    /// If the QC commits reconfiguration and starts a new epoch
    pub fn ends_epoch(&self) -> bool {
        self.signed_ledger_info.ledger_info().ends_epoch()
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        // Genesis's QC is implicitly agreed upon, it doesn't have real signatures.
        // If someone sends us a QC on a fake genesis, it'll fail to insert into BlockStore
        // because of the round constraint.

        // TODO: Earlier, we were comparing self.certified_block().round() to 0. Now, we are
        // comparing self.ledger_info().ledger_info().round() to 0. Is this okay?
        if self.ledger_info().ledger_info().round() == 0 {
            ensure!(
                self.ledger_info().get_num_voters() == 0,
                "Genesis QC should not carry signatures"
            );
            return Ok(());
        }
        self.ledger_info()
            .verify_signatures(validator)
            .context("Fail to verify WrappedLedgerInfo")?;
        Ok(())
    }

    pub fn create_merged_with_executed_state(
        &self,
        executed_ledger_info: LedgerInfoWithSignatures,
    ) -> anyhow::Result<WrappedLedgerInfo> {
        let self_commit_info = self.commit_info();
        let executed_commit_info = executed_ledger_info.ledger_info().commit_info();
        ensure!(
            self_commit_info.match_ordered_only(executed_commit_info),
            "Block info from QC and executed LI need to match, {:?} and {:?}",
            self_commit_info,
            executed_commit_info
        );
        Ok(Self::new(self.vote_data.clone(), executed_ledger_info))
    }
}
