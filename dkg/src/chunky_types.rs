// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::dkg::chunky_dkg::ChunkyDKGSessionMetadata;
use serde::{Deserialize, Serialize};

/// Reflection of `0x1::dkg::DKGStartEvent` in rust for Chunky DKG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkyDKGStartEvent {
    pub session_metadata: ChunkyDKGSessionMetadata,
    pub start_time_us: u64,
}

/// Reflection of Move type `0x1::dkg::DKGSessionState` for Chunky DKG.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChunkyDKGSessionState {
    pub metadata: ChunkyDKGSessionMetadata,
    pub start_time_us: u64,
    pub transcript: Vec<u8>,
}

impl ChunkyDKGSessionState {
    pub fn target_epoch(&self) -> u64 {
        self.metadata.dealer_epoch + 1
    }
}
