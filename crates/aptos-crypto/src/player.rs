// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines a struct to represents a participant (player) in a protocol.

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};

/// A validated identifier from 0 to n-1 for the n players involved in a secret
/// sharing / PVSS protocol.
///
/// `Player` deliberately does **not** derive `Serialize`/`Deserialize`: the whole
/// point of the type is that nobody can forge out-of-range player IDs. The only
/// way to obtain a `Player` is via a `TSecretSharingConfig` method, which
/// bounds-checks against the scheme's `n`. Over-the-wire encodings must use the
/// explicitly-untrusted [`RawPlayerIndex`] newtype below and convert via the
/// config.
#[derive(Copy, Debug, PartialEq, Eq, Clone)]
pub struct Player {
    id: usize,
}

impl Player {
    /// Returns the numeric ID of the player.
    pub fn get_id(&self) -> usize {
        self.id
    }

    /// Construct a `Player` without bounds-checking. Only for use inside
    /// `aptos-crypto` — the `TSecretSharingConfig` trait default methods need
    /// this, and they already enforce bounds. External crates must go through
    /// `TSecretSharingConfig::try_get_player` (or `get_player`).
    pub(crate) fn new_unchecked(id: usize) -> Self {
        Self { id }
    }
}

/// An untrusted, wire-format index for a player. This is what appears in
/// serialized network messages; it carries no safety guarantee on its own.
/// Use `TSecretSharingConfig::try_get_player_from_raw` to validate and convert
/// to a `Player`.
///
/// Under BCS this encodes byte-identically to a `usize`, and thus to the
/// pre-hardening `Player { id: usize }` encoding, so wire formats stay
/// compatible across versions.
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
    Hash,
)]
pub struct RawPlayerIndex(pub usize);

impl RawPlayerIndex {
    /// Returns the underlying index as a `usize`. Untrusted — use
    /// `TSecretSharingConfig::try_get_player_from_raw` for a validated value.
    pub fn get(&self) -> usize {
        self.0
    }
}

impl From<Player> for RawPlayerIndex {
    fn from(p: Player) -> Self {
        RawPlayerIndex(p.get_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `RawPlayerIndex` must be byte-identical to a bare `usize` under BCS,
    /// which is what the pre-hardening `Player { pub id: usize }` encoded as.
    /// If this ever regresses, wire formats containing `RawPlayerIndex` will
    /// break across versions.
    #[test]
    fn raw_player_index_bcs_is_byte_identical_to_usize() {
        for value in [0usize, 1, 7, 42, usize::MAX / 2] {
            let raw = RawPlayerIndex(value);
            let raw_bytes = bcs::to_bytes(&raw).unwrap();
            let usize_bytes = bcs::to_bytes(&value).unwrap();
            assert_eq!(
                raw_bytes, usize_bytes,
                "BCS encoding of RawPlayerIndex({}) diverged from encoding of {}: {:?} vs {:?}",
                value, value, raw_bytes, usize_bytes,
            );
        }
    }
}
