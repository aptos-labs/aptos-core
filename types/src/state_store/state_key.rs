// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

const ACCOUNT_ADDRESS_KEY_PREFIX: &str = "acc_blb_|";

#[derive(
    Clone, Debug, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum StateKey {
    AccountAddressKey(AccountAddress),
}

struct RawStateKey {
    bytes: Vec<u8>,
}

impl From<&StateKey> for RawStateKey {
    fn from(key: &StateKey) -> Self {
        match key {
            StateKey::AccountAddressKey(account_address) => {
                let mut account_address_prefix = ACCOUNT_ADDRESS_KEY_PREFIX.as_bytes().to_vec();
                account_address_prefix.extend(account_address.to_vec());
                RawStateKey {
                    bytes: account_address_prefix,
                }
            }
        }
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
