// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::Author;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{dkg::{DKGTranscriptWrapper, DKGPvssConfig}, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct DKGAggNodeMetadata {
    pub epoch: u64,
    pub author: Author,
}

impl DKGAggNodeMetadata {
    pub fn new(epoch: u64, author: Author) -> Self {
        Self { epoch, author }
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("[DKG] DKGAggNodeMetadata serialization failed!")
    }

    pub fn num_bytes(&self) -> usize {
        self.to_bytes().len()
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq, Eq)]
pub struct DKGAggNode {
    pub metadata: DKGAggNodeMetadata,
    pub agg_trx: DKGTranscriptWrapper,
}

impl DKGAggNode {
    pub fn new(epoch: u64, author: Author, agg_trx: DKGTranscriptWrapper) -> Self {
        Self {
            metadata: DKGAggNodeMetadata { epoch, author },
            agg_trx,
        }
    }

    pub fn metadata(&self) -> &DKGAggNodeMetadata {
        &self.metadata
    }

    pub fn author(&self) -> &Author {
        self.metadata.author()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn agg_trx(&self) -> &DKGTranscriptWrapper {
        &self.agg_trx
    }

    pub fn num_bytes(&self) -> usize {
        self.metadata.num_bytes() + self.agg_trx.num_bytes()
    }

    pub fn verify(&self, pvss_config: &DKGPvssConfig, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        let dealers = self.agg_trx.verify_dealers(verifier.len())?;
        let addresses = verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers.iter().filter_map(|&pos| addresses.get(pos)).cloned().collect::<Vec<_>>();
        // Ensure aggregated transcript has enough stakes
        verifier.check_voting_power(dealers_addresses.iter(), false)?;
        
        self.agg_trx.verify(pvss_config, verifier)
    }
}
