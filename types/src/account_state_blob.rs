// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::AccountResource, account_state::AccountState,
    state_store::state_value::StateValue,
};
use anyhow::{anyhow, format_err, Error, Result};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{arbitrary::Arbitrary, prelude::*};
use serde::{Deserialize, Deserializer, Serialize};
use std::{convert::TryFrom, fmt};

#[derive(Clone, Eq, PartialEq, Serialize, CryptoHasher)]
pub struct AccountStateBlob {
    pub blob: Vec<u8>,
    #[serde(skip)]
    hash: HashValue,
}

impl<'de> Deserialize<'de> for AccountStateBlob {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "AccountStateBlob")]
        struct RawBlob {
            blob: Vec<u8>,
        }
        let blob = RawBlob::deserialize(deserializer)?;

        Ok(Self::new(blob.blob))
    }
}

impl AccountStateBlob {
    fn new(blob: Vec<u8>) -> Self {
        let mut hasher = AccountStateBlobHasher::default();
        hasher.update(&blob);
        let hash = hasher.finish();
        Self { blob, hash }
    }
}

impl fmt::Debug for AccountStateBlob {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let decoded = bcs::from_bytes(&self.blob)
            .map(|account_state: AccountState| format!("{:#?}", account_state))
            .unwrap_or_else(|_| String::from("[fail]"));

        write!(
            f,
            "AccountStateBlob {{ \n \
             Raw: 0x{} \n \
             Decoded: {} \n \
             }}",
            hex::encode(&self.blob),
            decoded,
        )
    }
}

impl AsRef<[u8]> for AccountStateBlob {
    fn as_ref(&self) -> &[u8] {
        &self.blob
    }
}

impl From<&AccountStateBlob> for Vec<u8> {
    fn from(account_state_blob: &AccountStateBlob) -> Vec<u8> {
        account_state_blob.blob.clone()
    }
}

impl From<AccountStateBlob> for Vec<u8> {
    fn from(account_state_blob: AccountStateBlob) -> Vec<u8> {
        Self::from(&account_state_blob)
    }
}

impl From<Vec<u8>> for AccountStateBlob {
    fn from(blob: Vec<u8>) -> AccountStateBlob {
        AccountStateBlob::new(blob)
    }
}

impl TryFrom<&AccountState> for AccountStateBlob {
    type Error = Error;

    fn try_from(account_state: &AccountState) -> Result<Self> {
        Ok(Self::new(bcs::to_bytes(account_state)?))
    }
}

impl TryFrom<StateValue> for AccountStateBlob {
    type Error = Error;

    fn try_from(state_value: StateValue) -> Result<Self> {
        let bytes = state_value
            .maybe_bytes
            .ok_or_else(|| format_err!("Empty state value passed"))?;
        Ok(AccountStateBlob::from(bytes))
    }
}

impl TryFrom<&AccountResource> for AccountStateBlob {
    type Error = Error;

    fn try_from(account_resource: &AccountResource) -> Result<Self> {
        Self::try_from(&AccountState::try_from(account_resource)?)
    }
}

impl TryFrom<&AccountStateBlob> for AccountResource {
    type Error = Error;

    fn try_from(account_state_blob: &AccountStateBlob) -> Result<Self> {
        AccountState::try_from(account_state_blob)?
            .get_account_resource()?
            .ok_or_else(|| anyhow!("AccountResource not found."))
    }
}

impl CryptoHash for AccountStateBlob {
    type Hasher = AccountStateBlobHasher;

    fn hash(&self) -> HashValue {
        self.hash
    }
}

#[cfg(any(test, feature = "fuzzing"))]
prop_compose! {
    fn account_state_blob_strategy()(account_resource in any::<AccountResource>()) -> AccountStateBlob {
        AccountStateBlob::try_from(&account_resource).unwrap()
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for AccountStateBlob {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        account_state_blob_strategy().boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::state_value::StateValueWithProof;
    use bcs::test_helpers::assert_canonical_encode_decode;
    use proptest::collection::vec;

    fn hash_blob(blob: &[u8]) -> HashValue {
        let mut hasher = AccountStateBlobHasher::default();
        hasher.update(blob);
        hasher.finish()
    }

    proptest! {
        #[test]
        fn account_state_blob_hash(blob in vec(any::<u8>(), 1..100)) {
            prop_assert_eq!(hash_blob(&blob), AccountStateBlob::from(blob).hash());
        }

        #[test]
        fn account_state_blob_bcs_roundtrip(account_state_blob in any::<AccountStateBlob>()) {
            assert_canonical_encode_decode(account_state_blob);
        }

        #[test]
        fn account_state_with_proof_bcs_roundtrip(account_state_with_proof in any::<StateValueWithProof>()) {
            assert_canonical_encode_decode(account_state_with_proof);
        }
    }

    #[test]
    fn test_debug_does_not_panic() {
        format!("{:#?}", AccountStateBlob::from(vec![1u8, 2u8, 3u8]));
    }
}
