// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the Poseidon hash function for BN-254, which hashes $\le$ 16 field elements and
//! produces a single field element as output.
mod alt_fr;
mod constants;
pub mod keyless;

use crate::poseidon_bn254::constants::*;
use anyhow::bail;
use keyless::{
    pack_bytes_to_scalars, pad_and_pack_bytes_to_scalars_no_len,
    pad_and_pack_bytes_to_scalars_with_len,
};
use neptune::poseidon::{HashMode::OptimizedStatic, Poseidon};

/// The maximum number of input scalars that can be hashed using the Poseidon-BN254 hash function
/// exposed in `hash_scalars`.
pub const MAX_NUM_INPUT_SCALARS: usize = 16;

/// Macro for Poseidon-BN254-hashing a vector of scalars.
macro_rules! neptune_hash {
    ($elems:expr, $constants:expr) => {{
        let mut hasher = Poseidon::new(&$constants);
        hasher.reset();
        for elem in $elems.into_iter() {
            hasher.input(elem.into()).expect("Too many inputs");
        }
        hasher.hash_in_mode(OptimizedStatic);
        hasher.elements[0]
    }};
}

/// Given an array of up to `MAX_NUM_INPUT_SCALARS` field elements (in the BN254 scalar field), hashes
/// them using Poseidon-BN254 into a single field element.
pub fn hash_scalars(inputs: Vec<ark_bn254::Fr>) -> anyhow::Result<ark_bn254::Fr> {
    if inputs.is_empty() || inputs.len() > MAX_NUM_INPUT_SCALARS {
        bail!(
            "Poseidon-BN254 needs > 0 and <= {} inputs, but was called with {} inputs",
            MAX_NUM_INPUT_SCALARS,
            inputs.len()
        );
    }

    let result = match inputs.len() {
        1 => neptune_hash!(inputs, POSEIDON_1),
        2 => neptune_hash!(inputs, POSEIDON_2),
        3 => neptune_hash!(inputs, POSEIDON_3),
        4 => neptune_hash!(inputs, POSEIDON_4),
        5 => neptune_hash!(inputs, POSEIDON_5),
        6 => neptune_hash!(inputs, POSEIDON_6),
        7 => neptune_hash!(inputs, POSEIDON_7),
        8 => neptune_hash!(inputs, POSEIDON_8),
        9 => neptune_hash!(inputs, POSEIDON_9),
        10 => neptune_hash!(inputs, POSEIDON_10),
        11 => neptune_hash!(inputs, POSEIDON_11),
        12 => neptune_hash!(inputs, POSEIDON_12),
        13 => neptune_hash!(inputs, POSEIDON_13),
        14 => neptune_hash!(inputs, POSEIDON_14),
        15 => neptune_hash!(inputs, POSEIDON_15),
        16 => neptune_hash!(inputs, POSEIDON_16),
        _ => bail!(
            "Poseidon-BN254 was called with {} inputs, more than the maximum 16 allowed inputs.",
            inputs.len()
        ),
    };

    Ok(result.into())
}

/// Given a string and `max_bytes`, it pads the byte array of the string with zeros up to size `max_bytes`,
/// packs it to scalars, and returns the hash of the scalars.
pub fn pad_and_hash_string(str: &str, max_bytes: usize) -> anyhow::Result<ark_bn254::Fr> {
    pad_and_hash_bytes_with_len(str.as_bytes(), max_bytes)
}

/// Packs the bytes to a vector of scalars (see `pack_bytes_to_scalars`) and hashes the scalars via
/// `hash_scalars`.
///
/// Note: The byte packing encodes the length of the bytes properly so as to avoid collisions when
/// hashing, say, 0x00 versus 0x0000.
///
/// Note: We do not expose this function to avoid unnecessary bugs, since for SNARK circuits we
/// always have to hash padded byte arrays. If necessary, an expert developer can indirectly call
/// this via `pad_and_hash_bytes(bytes, bytes.len())`.
#[allow(unused)]
fn hash_bytes(bytes: &[u8]) -> anyhow::Result<ark_bn254::Fr> {
    let scalars = pack_bytes_to_scalars(bytes)?;
    hash_scalars(scalars)
}

/// Given `bytes`, if the length of `bytes` is less than `max_bytes`, pads `bytes` with zeros to length `max_bytes`.
/// Then it packs padded `bytes` and returns the hash of the scalars via `hash_scalars`.
///
/// This is used when we know that `bytes` will not terminate in 0's and may skip encoding the length, for
/// example ASCII strings. Otherwise, unexpected collisions can occur.
///
/// We do not expose this to minimize the risk of collisions due to improper use by the caller.
pub fn pad_and_hash_bytes_no_len(bytes: &[u8], max_bytes: usize) -> anyhow::Result<ark_bn254::Fr> {
    let scalars = pad_and_pack_bytes_to_scalars_no_len(bytes, max_bytes)?;
    hash_scalars(scalars)
}

/// Given `bytes`, if the length of `bytes` is less than `max_bytes`, pads `bytes` with zeros to length `max_bytes`.
/// Then it packs these padded `bytes` and preserves the original length as the first scalar and returns the hash of the scalars via `hash_scalars`.
///
/// This is used when we want to preserve the length of the `bytes` to prevent collisions where `bytes` could terminate in 0's.
pub fn pad_and_hash_bytes_with_len(
    bytes: &[u8],
    max_bytes: usize,
) -> anyhow::Result<ark_bn254::Fr> {
    let scalars = pad_and_pack_bytes_to_scalars_with_len(bytes, max_bytes)?;
    hash_scalars(scalars)
}

#[cfg(test)]
mod test {
    use crate::poseidon_bn254::hash_scalars;
    use std::str::FromStr;

    #[test]
    fn test_poseidon_bn254_poseidon_ark_vectors() {
        let mut inputs = vec!["1", "2"]
            .into_iter()
            .map(|hex| ark_bn254::Fr::from_str(hex).unwrap())
            .collect::<Vec<ark_bn254::Fr>>();

        // From https://github.com/arnaucube/poseidon-ark/blob/6d2487aa1308d9d3860a2b724c485d73095c1c68/src/lib.rs#L170
        let h = hash_scalars(inputs.clone()).unwrap();
        assert_eq!(
            h.to_string(),
            "7853200120776062878684798364095072458815029376092732009249414926327459813530"
        );

        // From the same place.
        inputs.pop();
        let h = hash_scalars(inputs).unwrap();
        assert_eq!(
            h.to_string(),
            "18586133768512220936620570745912940619677854269274689475585506675881198879027"
        );
    }
}
