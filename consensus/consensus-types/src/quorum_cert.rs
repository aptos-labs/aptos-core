// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{vote_data::VoteData, wrapped_ledger_info::WrappedLedgerInfo};
use anyhow::{ensure, Context};
use aptos_bitvec::BitVec;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
pub struct QuorumCert {
    /// The vote information is certified by the quorum.
    vote_data: VoteData,
    /// The signed LedgerInfo of a committed block that carries the data about the certified block.
    signed_ledger_info: LedgerInfoWithSignatures,
}

impl Display for QuorumCert {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "QuorumCert: [{}, {}]",
            self.vote_data, self.signed_ledger_info
        )
    }
}

impl QuorumCert {
    pub fn new(vote_data: VoteData, signed_ledger_info: LedgerInfoWithSignatures) -> Self {
        QuorumCert {
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
        self.vote_data().proposed()
    }

    pub fn parent_block(&self) -> &BlockInfo {
        self.vote_data().parent()
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

    /// QuorumCert for the genesis block deterministically generated from end-epoch LedgerInfo:
    /// - the ID of the block is determined by the generated genesis block.
    /// - the accumulator root hash of the LedgerInfo is set to the last executed state of previous
    ///   epoch.
    /// - the map of signatures is empty because genesis block is implicitly agreed.
    pub fn certificate_for_genesis_from_ledger_info(
        ledger_info: &LedgerInfo,
        genesis_id: HashValue,
    ) -> QuorumCert {
        let ancestor = BlockInfo::new(
            ledger_info
                .epoch()
                .checked_add(1)
                .expect("Integer overflow when creating cert for genesis from ledger info"),
            0,
            genesis_id,
            ledger_info.transaction_accumulator_hash(),
            ledger_info.version(),
            ledger_info.timestamp_usecs(),
            None,
        );

        let vote_data = VoteData::new(ancestor.clone(), ancestor.clone());
        let li = LedgerInfo::new(ancestor, vote_data.hash());

        let validator_set_size = ledger_info
            .next_epoch_state()
            .expect("Next epoch state not found in ledger info")
            .verifier
            .len();

        QuorumCert::new(
            vote_data,
            LedgerInfoWithSignatures::new(
                li,
                AggregateSignature::new(BitVec::with_num_bits(validator_set_size as u16), None),
            ),
        )
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        let vote_hash = self.vote_data.hash();
        ensure!(
            self.ledger_info().ledger_info().consensus_data_hash() == vote_hash,
            "Quorum Cert's hash mismatch LedgerInfo"
        );
        // Genesis's QC is implicitly agreed upon, it doesn't have real signatures.
        // If someone sends us a QC on a fake genesis, it'll fail to insert into BlockStore
        // because of the round constraint.
        if self.certified_block().round() == 0 {
            ensure!(
                self.parent_block() == self.certified_block(),
                "Genesis QC has inconsistent parent block with certified block"
            );
            ensure!(
                self.certified_block() == self.ledger_info().ledger_info().commit_info(),
                "Genesis QC has inconsistent commit block with certified block"
            );
            ensure!(
                self.ledger_info().get_num_voters() == 0,
                "Genesis QC should not carry signatures"
            );
            return Ok(());
        }
        self.ledger_info()
            .verify_signatures(validator)
            .context("Fail to verify QuorumCert")?;
        self.vote_data.verify()?;
        Ok(())
    }

    pub fn create_merged_with_executed_state(
        &self,
        executed_ledger_info: LedgerInfoWithSignatures,
    ) -> anyhow::Result<QuorumCert> {
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

    pub fn into_wrapped_ledger_info(&self) -> WrappedLedgerInfo {
        WrappedLedgerInfo::new(self.vote_data.clone(), self.ledger_info().clone())
    }
}
