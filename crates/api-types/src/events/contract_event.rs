// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::on_chain_config::jwks::ProviderJWKs;
use crate::on_chain_config::dkg::DKGStartEvent;


#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub enum GravityEvent {
    NewEpoch(u64, Bytes),
    ObservedJWKsUpdated(u64, Vec<ProviderJWKs>),
    DKG(DKGStartEvent),
}