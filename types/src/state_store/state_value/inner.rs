// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_value::{metadata::StateValueMetadataExt, StateValueMetadata};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(
    BCSCryptoHash,
    Clone,
    CryptoHasher,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    Ord,
    PartialOrd,
    Hash,
)]
#[serde(rename = "StateValue")]
pub enum StateValueInner {
    V0(Bytes),
    WithMetadata {
        data: Bytes,
        metadata: StateValueMetadata,
    },
}

impl StateValueInner {
    pub(crate) fn metadata(&self) -> Option<StateValueMetadata> {
        match self {
            StateValueInner::V0(_) => None,
            StateValueInner::WithMetadata { metadata, .. } => Some(metadata.clone()),
        }
    }

    pub(crate) fn metadata_ext(&self) -> Option<StateValueMetadataExt> {
        match self {
            StateValueInner::V0(_) => None,
            StateValueInner::WithMetadata { data, metadata } => Some(StateValueMetadataExt {
                inner: metadata.clone(),
                num_bytes: data.len(),
            }),
        }
    }
}
