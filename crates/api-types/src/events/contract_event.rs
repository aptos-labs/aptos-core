// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};


#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum GravityEvent {
    NewEpoch(u64),
    JWK,
    DKG,
}