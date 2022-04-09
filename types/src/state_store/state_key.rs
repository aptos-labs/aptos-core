// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::access_path::AccessPath;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use move_core_types::account_address::AccountAddress;
use num_derive::ToPrimitive;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum StateKey {
    AccountAddressKey(AccountAddress),
    AccessPath(AccessPath),
    // Only used for testing
    #[serde(with = "serde_bytes")]
    Raw(Vec<u8>),
}

#[derive(ToPrimitive)]
enum StateKeyPrefix {
    AccountAddress,
    AccessPath,
    Raw = 255,
}

impl StateKeyPrefix {
    fn to_bytes(&self) -> Vec<u8> {
        let byte = self
            .to_u8()
            .expect("Failed to convert StateKeyPrefix to u8");
        vec![byte]
    }
}

pub struct RawStateKey {
    pub bytes: Vec<u8>,
}

impl From<&StateKey> for RawStateKey {
    fn from(key: &StateKey) -> Self {
        let (prefix, raw_key) = match key {
            StateKey::AccountAddressKey(account_address) => {
                (StateKeyPrefix::AccountAddress, account_address.to_vec())
            }
            StateKey::AccessPath(access_path) => {
                let mut raw_key = access_path.address.to_vec();
                raw_key.extend(access_path.path.clone());
                (StateKeyPrefix::AccessPath, raw_key)
            }
            StateKey::Raw(raw_bytes) => (StateKeyPrefix::Raw, raw_bytes.to_vec()),
        };
        let mut bytes = prefix.to_bytes();
        bytes.extend(raw_key);

        Self { bytes }
    }
}

impl CryptoHash for StateKey {
    type Hasher = StateKeyHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(RawStateKey::from(self).bytes.as_ref());
        state.finish()
    }
}
