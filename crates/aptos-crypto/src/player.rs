// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Defines a struct to represents a participant (player) in a protocol.

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};

/// An identifier from 0 to n-1 for the n players involved in the PVSS protocol.
#[derive(
    CanonicalSerialize,
    CanonicalDeserialize,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct Player {
    /// A number from 0 to n-1.
    pub id: usize,
}

impl Player {
    /// Returns the numeric ID of the player.
    pub fn get_id(&self) -> usize {
        self.id
    }
}
