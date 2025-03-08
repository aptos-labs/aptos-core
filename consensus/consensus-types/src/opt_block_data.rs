// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_data::BlockType,
    common::{Author, Payload, Round},
    quorum_cert::QuorumCert,
};
use anyhow::{bail, ensure, Result};
use aptos_crypto::HashValue;
use aptos_crypto_derive::CryptoHasher;
use aptos_infallible::duration_since_epoch;
use aptos_types::validator_txn::ValidatorTransaction;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, CryptoHasher)]
/// Same as BlockData, without QC and with parent id
pub struct OptBlockData {
    pub epoch: u64,
    pub round: Round,
    pub timestamp_usecs: u64,
    pub parent_id: HashValue,
    pub block_type: BlockType,
}

impl OptBlockData {
    pub fn author(&self) -> &Author {
        match &self.block_type {
            BlockType::OptProposal { author, .. } => author,
            _ => panic!("Invalid block type"),
        }
    }

    pub fn block_type(&self) -> &BlockType {
        &self.block_type
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn parent_id(&self) -> HashValue {
        self.parent_id
    }

    pub fn grandparent_qc(&self) -> Result<QuorumCert> {
        match &self.block_type {
            BlockType::OptProposal { grandparent_qc, .. } => Ok(grandparent_qc.clone()),
            _ => bail!("Invalid block type"),
        }
    }

    pub fn payload(&self) -> Option<&Payload> {
        match &self.block_type {
            BlockType::OptProposal { payload, .. } => Some(payload),
            _ => panic!("Invalid block type"),
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match &self.block_type {
            BlockType::OptProposal { validator_txns, .. } => {
                (!validator_txns.is_empty()).then_some(validator_txns)
            },
            _ => panic!("Invalid block type"),
        }
    }

    pub fn dag_nodes(&self) -> Option<&Vec<HashValue>> {
        if let BlockType::DAGBlock {
            node_digests: nodes_digests,
            ..
        } = &self.block_type
        {
            Some(nodes_digests)
        } else {
            None
        }
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.timestamp_usecs
    }

    pub fn is_opt_proposal(&self) -> bool {
        matches!(self.block_type, BlockType::OptProposal { .. })
    }

    pub fn new_proposal(
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        parent_id: HashValue,
        grandparent_qc: QuorumCert,
    ) -> Self {
        Self {
            epoch,
            round,
            timestamp_usecs,
            parent_id,
            block_type: BlockType::OptProposal {
                validator_txns,
                payload,
                author,
                failed_authors,
                grandparent_qc,
            },
        }
    }

    pub fn verify(&self) -> Result<()> {
        // Verifies that the OptBlockData is well-formed.
        ensure!(
            self.is_opt_proposal(),
            "Only optimistic proposal is supported"
        );

        if let Some(payload) = self.payload() {
            payload.verify_epoch(self.epoch())?;
        }

        let current_ts = duration_since_epoch();

        // we can say that too far is 5 minutes in the future
        const TIMEBOUND: u64 = 300_000_000;
        ensure!(
            self.timestamp_usecs() <= (current_ts.as_micros() as u64).saturating_add(TIMEBOUND),
            "Blocks must not be too far in the future"
        );

        Ok(())
    }
}
