// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_value::StateValueMetadata;
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
