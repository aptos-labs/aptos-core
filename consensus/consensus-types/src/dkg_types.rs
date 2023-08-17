// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::Author;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::dkg::{DKGTranscriptWrapper, DKGPvssConfig};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct DKGAggNodeMetadata {
    pub epoch: u64,
    pub author: Author,
}

impl DKGAggNodeMetadata {
    #[cfg(test)]
    pub fn new_for_test(epoch: u64, author: Author) -> Self {
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
    // dkg todo: use aggregated transcript here
    // I am assuming aggregated transcript contains the authors of individual transcript
    pub agg_trx: DKGTranscriptWrapper,
}

impl DKGAggNode {
    pub fn new(epoch: u64, author: Author, agg_trx: DKGTranscriptWrapper) -> Self {
        Self {
            metadata: DKGAggNodeMetadata { epoch, author },
            agg_trx,
        }
    }

    #[cfg(test)]
    pub fn new_for_test(metadata: DKGAggNodeMetadata, agg_trx: DKGTranscriptWrapper) -> Self {
        Self { metadata, agg_trx }
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

    pub fn verify(&self, pvss_config: &DKGPvssConfig) -> anyhow::Result<()> {
        self.agg_trx.verify(pvss_config)
    }
}
