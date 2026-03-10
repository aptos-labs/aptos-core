// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

/// The point of Player is to provide type-safety: ensure nobody creates out-of-range player IDs.
/// So there is no `new()` method; only the SecretSharingConfig trait is allowed to create them.
// TODO: AFAIK the only way to really enforce this is to put both traits inside the same module (or use unsafe Rust)
impl Player {
    /// Returns the numeric ID of the player.
    pub fn get_id(&self) -> usize {
        self.id
    }
}
