// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ValidatorTransaction {
    DummyTopic1(DummyValidatorTransaction),
    DummyTopic2(DummyValidatorTransaction),}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct DummyValidatorTransaction {
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl ValidatorTransaction {
    pub fn dummy1(payload: Vec<u8>) -> Self {
        Self::DummyTopic1(DummyValidatorTransaction { payload })
    }

    pub fn dummy2(payload: Vec<u8>) -> Self {
        Self::DummyTopic2(DummyValidatorTransaction { payload })
    }

    pub fn size_in_bytes(&self) -> usize {
        bcs::to_bytes(self).unwrap().len()
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Topic {
    RANDOMNESS_DKG = 0,
    DUMMY1,
    DUMMY2,
}
