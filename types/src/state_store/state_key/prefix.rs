// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_key::{inner::StateKeyTag, StateKey};
use move_core_types::account_address::AccountAddress;

// Struct for defining prefix of a state key, which can be used for finding all the values with a
// particular key prefix
#[derive(Clone, Debug)]
pub struct StateKeyPrefix {
    tag: StateKeyTag,
    bytes: Vec<u8>,
}

impl StateKeyPrefix {
    pub fn new(tag: StateKeyTag, bytes: Vec<u8>) -> Self {
        Self { tag, bytes }
    }

    /// Serializes to bytes for physical storage.
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let mut out = vec![self.tag.clone() as u8];
        out.extend(self.bytes.clone());
        Ok(out)
    }

    /// Checks if the current prefix is a valid prefix of a particular state_key
    pub fn is_prefix(&self, state_key: &StateKey) -> anyhow::Result<bool> {
        let encoded_key = state_key.encoded();
        let encoded_prefix = self.encode()?;
        // Check if bytes is a sub-vector of encoded key.
        if encoded_prefix.len() > encoded_key.len() {
            return Ok(false);
        }
        Ok(encoded_prefix == encoded_key[..encoded_prefix.len()])
    }
}

impl From<AccountAddress> for StateKeyPrefix {
    fn from(address: AccountAddress) -> Self {
        Self::new(StateKeyTag::AccessPath, address.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        account_config::{AccountResource, CoinStoreResource},
        state_store::state_key::{inner::StateKeyTag, prefix::StateKeyPrefix, StateKey},
        AptosCoinType,
    };
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn test_state_key_prefix() {
        let address1 = AccountAddress::new([12u8; AccountAddress::LENGTH]);
        let address2 = AccountAddress::new([22u8; AccountAddress::LENGTH]);
        let key1 = StateKey::resource_typed::<AccountResource>(&address1).unwrap();
        let key2 = StateKey::resource_typed::<CoinStoreResource<AptosCoinType>>(&address2).unwrap();

        let account1_key_prefx = StateKeyPrefix::new(StateKeyTag::AccessPath, address1.to_vec());
        let account2_key_prefx = StateKeyPrefix::new(StateKeyTag::AccessPath, address2.to_vec());

        assert!(account1_key_prefx.is_prefix(&key1).unwrap());
        assert!(account2_key_prefx.is_prefix(&key2).unwrap());

        assert!(!account1_key_prefx.is_prefix(&key2).unwrap());
        assert!(!account2_key_prefx.is_prefix(&key1).unwrap());
    }
}
