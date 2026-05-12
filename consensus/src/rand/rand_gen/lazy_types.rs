// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lazy wire types for randomness `AugmentedData`.
//!
//! `LazyG1` and `LazyDelta` carry the same on-wire BCS encoding as
//! `blstrs::G1Projective` and `aptos_dkg::weighted_vuf::pinkas::RandomizedPKs`
//! respectively, but skip the expensive subgroup-checked decompression at
//! deserialization time. This lets `AugmentedData::verify` perform cheap
//! structural checks (length match, `fast_delta == None`) before paying the
//! per-element decompression cost — bounding the per-message CPU work that a
//! peer-supplied payload can force on the receiver.

use anyhow::{anyhow, ensure};
use aptos_types::randomness::Delta;
use blstrs::{G1Affine, G1Projective};
use group::Curve;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

/// Compressed-bytes form of a `G1Projective`. Wire format (48 raw bytes) is
/// identical to `blstrs::G1Projective`'s BCS encoding via `serialize_tuple(48)`.
/// Unlike `G1Projective::deserialize`, decoding does not subgroup-check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LazyG1(#[serde(with = "BigArray")] [u8; 48]);

impl LazyG1 {
    /// Compress a known-valid `G1Projective` into its 48-byte wire form.
    pub fn from_decompressed(g: &G1Projective) -> Self {
        Self(g.to_affine().to_compressed())
    }

    /// Subgroup-checked decompression. This is the expensive operation we
    /// defer until a payload has cleared structural validation.
    pub fn decompress(&self) -> anyhow::Result<G1Projective> {
        let affine: Option<G1Affine> = G1Affine::from_compressed(&self.0).into();
        affine
            .map(G1Projective::from)
            .ok_or_else(|| anyhow!("invalid G1: subgroup check failed"))
    }
}

/// Wire-compatible lazy form of `aptos_dkg::weighted_vuf::pinkas::RandomizedPKs`
/// (= `aptos_types::randomness::Delta`). Holds compressed bytes; decompression
/// happens explicitly via [`LazyDelta::validate_and_decompress`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LazyDelta {
    pi: LazyG1,
    rks: Vec<LazyG1>,
}

impl LazyDelta {
    pub fn from_decompressed(delta: &Delta) -> Self {
        Self {
            pi: LazyG1::from_decompressed(delta.pi()),
            rks: delta.rks().iter().map(LazyG1::from_decompressed).collect(),
        }
    }

    /// Verify the structural invariant (`rks.len() == expected_len`) cheaply,
    /// then perform subgroup-checked decompression on every element. Returns
    /// the decompressed `Delta` on success.
    ///
    /// On a malformed payload (length mismatch or any element failing subgroup
    /// check), no decompression happens past the failing element — bounding
    /// the work an attacker can force.
    pub fn validate_and_decompress(&self, expected_len: usize) -> anyhow::Result<Delta> {
        ensure!(
            self.rks.len() == expected_len,
            "delta.rks length {} does not match expected {}",
            self.rks.len(),
            expected_len,
        );
        let pi = self.pi.decompress()?;
        let rks = self
            .rks
            .iter()
            .map(LazyG1::decompress)
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Delta::from_decompressed(pi, rks))
    }

    /// Construct a `LazyDelta` directly from compressed bytes. Test-only —
    /// production paths should go through `from_decompressed`. Used to
    /// fabricate malformed wire payloads in regression tests.
    #[cfg(test)]
    pub fn from_raw_bytes_for_test(pi: LazyG1, rks: Vec<LazyG1>) -> Self {
        Self { pi, rks }
    }
}

impl LazyG1 {
    /// Construct from arbitrary bytes (no subgroup check). Test-only —
    /// production paths should go through `from_decompressed`.
    #[cfg(test)]
    pub fn from_raw_bytes_for_test(bytes: [u8; 48]) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blstrs::{G1Projective, Scalar};
    use group::Group;

    /// Generate a deterministic distinct G1 point: `generator * (i+1)`.
    fn test_point(i: u64) -> G1Projective {
        G1Projective::generator() * Scalar::from(i + 1)
    }

    /// LazyG1 must round-trip BCS-bitwise-identical to G1Projective so that
    /// validators on either type can interoperate on the wire.
    #[test]
    fn lazy_g1_wire_compat_with_g1_projective() {
        for i in 0..16 {
            let g = test_point(i);
            let lazy = LazyG1::from_decompressed(&g);

            // Same on-wire bytes as the underlying G1Projective.
            let bytes_g = bcs::to_bytes(&g).unwrap();
            let bytes_lazy = bcs::to_bytes(&lazy).unwrap();
            assert_eq!(bytes_g, bytes_lazy, "BCS encoding must match G1Projective");
            assert_eq!(bytes_g.len(), 48);

            // Round-trip: bytes from G1Projective decode as LazyG1 (no subgroup check).
            let decoded_lazy: LazyG1 = bcs::from_bytes(&bytes_g).unwrap();
            assert_eq!(decoded_lazy, lazy);

            // And decompression yields the original point.
            let decompressed = decoded_lazy.decompress().unwrap();
            assert_eq!(decompressed, g);
        }
    }

    /// LazyDelta must round-trip BCS-bitwise-identical to RandomizedPKs.
    #[test]
    fn lazy_delta_wire_compat_with_delta() {
        let pi = test_point(0);
        let rks: Vec<_> = (1..8).map(test_point).collect();
        let delta = Delta::from_decompressed(pi, rks);
        let lazy = LazyDelta::from_decompressed(&delta);

        let bytes_delta = bcs::to_bytes(&delta).unwrap();
        let bytes_lazy = bcs::to_bytes(&lazy).unwrap();
        assert_eq!(bytes_delta, bytes_lazy);

        let decoded: LazyDelta = bcs::from_bytes(&bytes_delta).unwrap();
        let recovered = decoded.validate_and_decompress(7).unwrap();
        assert_eq!(recovered, delta);
    }

    #[test]
    fn validate_and_decompress_rejects_length_mismatch() {
        let pi = test_point(0);
        let rks: Vec<_> = (1..6).map(test_point).collect();
        let delta = Delta::from_decompressed(pi, rks);
        let lazy = LazyDelta::from_decompressed(&delta);

        // Expected length 7 but actual is 5 — must fail before any decompression.
        let err = lazy.validate_and_decompress(7).unwrap_err();
        assert!(err.to_string().contains("length"));
    }

    /// Explicit attack-shape regression: decoding a malformed payload with a
    /// huge `rks` length, then mismatched expected length, must reject without
    /// performing all 1M subgroup checks. We verify by checking that decode +
    /// length-check returns Err quickly without ever calling decompress on
    /// most elements.
    #[test]
    fn validate_rejects_oversized_rks_before_decompression() {
        // Construct a LazyDelta with garbage bytes that LOOKS like it has many rks.
        // Use random 48-byte blobs (most won't be valid G1 points). The point of
        // this test is that validate_and_decompress fails on the length mismatch
        // BEFORE attempting to subgroup-check any element.
        let pi_bytes = [0u8; 48];
        // 1024 garbage entries — would cost real CPU if we decompressed each.
        let rks: Vec<LazyG1> = (0..1024).map(|_| LazyG1([0u8; 48])).collect();
        let lazy = LazyDelta {
            pi: LazyG1(pi_bytes),
            rks,
        };

        // Expected length is 7; actual is 1024. Must error on length check, not
        // attempt to decompress 1024 garbage elements.
        let start = std::time::Instant::now();
        let err = lazy.validate_and_decompress(7).unwrap_err();
        let elapsed = start.elapsed();

        assert!(err.to_string().contains("length"));
        assert!(
            elapsed.as_millis() < 50,
            "length-check rejection took {}ms; expected <50ms (no per-element decompression)",
            elapsed.as_millis(),
        );
    }
}
