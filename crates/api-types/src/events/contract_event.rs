// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use serde::{Deserialize, Serialize};


#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub enum GravityEvent {
    NewEpoch(u64, Bytes),
    JWK,
    DKG,
}