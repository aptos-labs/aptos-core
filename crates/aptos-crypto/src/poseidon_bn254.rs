// Copyright © Aptos Foundation

//! Implements the Poseidon hash function for BN-254, which hashes $\le$ 16 field elements and
//! produces a single field element as output.
use anyhow::bail;
use ark_ff::PrimeField;
// TODO(zkid): Figure out the right library for Poseidon.
use poseidon_ark::Poseidon;

/// The maximum number of input scalars that can be hashed using the Poseidon-BN254 hash function
/// exposed in `hash_scalars`.
pub const MAX_NUM_INPUT_SCALARS: usize = 16;

/// A BN254 scalar is 254 bits which means it can only store up to 31 bytes of data. We could use a
/// more complicated packing to take advantage of the unused 6 bits, but we do not since it allows
/// us to keep our SNARK circuits simpler.
pub const BYTES_PACKED_PER_SCALAR: usize = 31;

/// The maximum number of bytes that can be given as input to the byte-oriented variant of the
/// Poseidon-BN254 hash function exposed in `pad_and_hash_bytes`.
///
/// Note: The first scalar is used to encode the length of the byte array. The max. # of bytes that
/// can be stored in 16 scalars is 16 * 31 = 496 bytes. So the size can be encoded into
/// `ceil(log_2(496)) = 9` bits of a scalar. That would leave 254 - 9 = 245 bits > 30 bytes for
/// storing data in that scalar. We do not plan on exploiting this extra free space (since our
/// SNARK circuits would have to implement this more complicated packing).
pub const MAX_NUM_INPUT_BYTES: usize = MAX_NUM_INPUT_SCALARS * BYTES_PACKED_PER_SCALAR;

/// Given an array of up to `MAX_NUM_INPUT_SCALARS` field elements (in the BN254 scalar field), hashes
/// them using Poseidon-BN254 into a single field element.
pub fn hash_scalars(inputs: Vec<ark_bn254::Fr>) -> anyhow::Result<ark_bn254::Fr> {
    if inputs.is_empty() || inputs.len() > MAX_NUM_INPUT_SCALARS {
        bail!(
            "Poseidon-BN254 needs > 0 and <= 16 inputs, but was called with {} inputs",
            inputs.len()
        );
    }

    let hash = Poseidon::new();

    hash.hash(inputs).map_err(anyhow::Error::msg)
}

/// Given an string and `max_bytes`, it pads the byte array of the string with zeros up to size `max_bytes`,
/// packs it to scalars, and returns the hash of the scalars.
///
/// This function calls `pad_and_pack_bytes_to_scalars_no_len` safely as strings will not contain the zero byte except to terminate.
pub fn pad_and_hash_string(str: &str, max_bytes: usize) -> anyhow::Result<ark_bn254::Fr> {
    pad_and_hash_bytes_with_len(str.as_bytes(), max_bytes)
}

/// Given $n$ bytes, this function returns $k$ field elements that pack those bytes as tightly as
/// possible.  This will not store the length $n$.
///
/// We pack the $i$th chunk of 31 bytes into $e_(i-1)$, assuming a little-endian (LE) encoding.
///
/// If the last chunk is smaller than 31 bytes, so all remaining bytes in its associated field
/// element are padded to zero due to the LE encoding.
fn pack_bytes_to_scalars(bytes: &[u8]) -> anyhow::Result<Vec<ark_bn254::Fr>> {
    if bytes.len() > MAX_NUM_INPUT_BYTES {
        bail!(
            "Cannot hash more than {} bytes. Was given {} bytes.",
            MAX_NUM_INPUT_BYTES,
            bytes.len()
        );
    }

    let scalars = bytes
        .chunks(BYTES_PACKED_PER_SCALAR)
        .map(|chunk| pack_bytes_to_one_scalar(chunk).expect("chunk converts to scalar"))
        .collect::<Vec<ark_bn254::Fr>>();

    Ok(scalars)
}

/// Given $n$ bytes, this function left pads bytes with 'max_bytes'- $n$ zeros and returns $k+1$ field elements that pack those bytes as tightly as
/// possible where $e_(0)$ is $n$ and $k$ is the ceiling of `max_bytes`/`BYTES_PACKED_PER_SCALAR`.
pub fn pad_and_pack_bytes_to_scalars_with_len(
    bytes: &[u8],
    max_bytes: usize,
) -> anyhow::Result<Vec<ark_bn254::Fr>> {
    let len = bytes.len();
    if max_bytes > MAX_NUM_INPUT_BYTES {
        bail!(
            "Cannot hash more than {} bytes. Was given {} bytes.",
            MAX_NUM_INPUT_BYTES,
            len
        );
    }
    if len > max_bytes {
        bail!(
            "Byte array length of {} is NOT <= max length of {} bytes.",
            bytes.len(),
            max_bytes
        );
    }

    let len_scalar = pack_bytes_to_one_scalar(&len.to_le_bytes())?;
    let scalars = pad_and_pack_bytes_to_scalars_no_len(bytes, max_bytes)?
        .into_iter()
        .chain([len_scalar])
        .collect::<Vec<ark_bn254::Fr>>();
    Ok(scalars)
}

/// Given $n$ bytes, this function left pads bytes with 'max_bytes'- $n$ zeros and returns $k$ field elements that pack those bytes as tightly as
/// possible, where $k$ is the ceiling of `max_bytes`/`BYTES_PACKED_PER_SCALAR`.
fn pad_and_pack_bytes_to_scalars_no_len(
    bytes: &[u8],
    max_bytes: usize,
) -> anyhow::Result<Vec<ark_bn254::Fr>> {
    let len = bytes.len();
    if max_bytes > MAX_NUM_INPUT_BYTES {
        bail!(
            "Cannot hash more than {} bytes. Was given {} bytes.",
            MAX_NUM_INPUT_BYTES,
            len
        );
    }
    if bytes.len() > max_bytes {
        bail!(
            "Byte array length of {} is NOT <= max length of {} bytes.",
            bytes.len(),
            max_bytes
        );
    }

    let padded = zero_pad_bytes(bytes, max_bytes)?;
    let scalars = pack_bytes_to_scalars(padded.as_slice())?;
    Ok(scalars)
}

/// Packs the bytes to a vector of scalars (see `pack_bytes_to_scalars`) and hashes the scalars via
/// `hash_scalars`.
///
/// Note: The byte packing encodes the length of the bytes properly so as to avoid collisions when
/// hashing, say, 0x00 versus 0x0000.
///
/// WARNING: We do not expose this function to avoid unnecessary bugs, since for SNARK circuits we
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
/// example ASCII strings. Otherwise unexpected collisions can occur.
///
/// Due to risk of collisions due to improper use by the caller, it is not exposed.
#[allow(unused)]
fn pad_and_hash_bytes_no_len(bytes: &[u8], max_bytes: usize) -> anyhow::Result<ark_bn254::Fr> {
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

/// We often have to pad byte arrays with 0s, up to a maximum length.
/// Given an array of bytes in `bytes`, if its length is less than `size`, appends zero bytes to
/// it until its length is equal to `size`.
fn zero_pad_bytes(bytes: &[u8], size: usize) -> anyhow::Result<Vec<u8>> {
    if size > MAX_NUM_INPUT_BYTES {
        bail!(
            "Cannot pad to more than {} bytes. Requested size is {}.",
            MAX_NUM_INPUT_BYTES,
            size
        );
    }

    if bytes.len() > size {
        bail!("Cannot pad {} byte(s) to size {}", bytes.len(), size);
    }

    let mut padded = bytes.to_vec();
    padded.resize(size, 0x00);
    Ok(padded)
}

/// Converts the chunk of bytes into a scalar, assuming it is of size less than or equal to `BYTES_PACKED_PER_SCALAR`.
pub fn pack_bytes_to_one_scalar(chunk: &[u8]) -> anyhow::Result<ark_bn254::Fr> {
    if chunk.len() > BYTES_PACKED_PER_SCALAR {
        bail!(
            "Cannot convert chunk to scalar. Max chunk size is {} bytes. Was given {} bytes.",
            BYTES_PACKED_PER_SCALAR,
            chunk.len(),
        );
    }
    let fr = ark_bn254::Fr::from_le_bytes_mod_order(chunk);
    Ok(fr)
}

#[cfg(test)]
mod test {
    use crate::{
        poseidon_bn254,
        poseidon_bn254::{
            pack_bytes_to_scalars, BYTES_PACKED_PER_SCALAR, MAX_NUM_INPUT_BYTES,
            MAX_NUM_INPUT_SCALARS,
        },
    };
    use ark_ff::{BigInteger, One, PrimeField, Zero};
    use num_bigint::BigUint;
    use std::str::FromStr;

    #[test]
    fn test_poseidon_bn254_poseidon_ark_vectors() {
        let mut inputs = vec!["1", "2"]
            .into_iter()
            .map(|hex| ark_bn254::Fr::from_str(hex).unwrap())
            .collect::<Vec<ark_bn254::Fr>>();

        // From https://github.com/arnaucube/poseidon-ark/blob/6d2487aa1308d9d3860a2b724c485d73095c1c68/src/lib.rs#L170
        let h = poseidon_bn254::hash_scalars(inputs.clone()).unwrap();
        assert_eq!(
            h.to_string(),
            "7853200120776062878684798364095072458815029376092732009249414926327459813530"
        );

        // From the same place.
        inputs.pop();
        let h = poseidon_bn254::hash_scalars(inputs).unwrap();
        assert_eq!(
            h.to_string(),
            "18586133768512220936620570745912940619677854269274689475585506675881198879027"
        );
    }

    #[test]
    fn test_poseidon_bn254_pad_and_hash_bytes() {
        let aud = "google";
        const MAX_AUD_VAL_BYTES: usize = 248;
        let aud_val_hash = poseidon_bn254::pad_and_hash_string(aud, MAX_AUD_VAL_BYTES).unwrap();
        assert_eq!(
            aud_val_hash.to_string(),
            "4022319167392179362271493931675371567039199401695470709241660273812313544045"
        );
    }

    #[test]
    fn test_poseidon_bn254_pad_and_hash_bytes_no_collision() {
        let s1: [u8; 3] = [0, 0, 1];
        let s2: [u8; 4] = [0, 0, 1, 0];
        const MAX_BYTES: usize = 248;
        let h1 = poseidon_bn254::pad_and_hash_bytes_with_len(&s1, MAX_BYTES).unwrap();
        let h2 = poseidon_bn254::pad_and_hash_bytes_with_len(&s2, MAX_BYTES).unwrap();

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_poseidon_bn254_pack_bytes() {
        // b"" -> vec![Fr(0)]
        let scalars = pack_bytes_to_scalars(b"").unwrap();
        assert_eq!(scalars.len(), 0);

        // 0x01 -> vec![Fr(0)]
        let scalars = pack_bytes_to_scalars(vec![0x01].as_slice()).unwrap();
        assert_eq!(scalars.len(), 1);
        assert_eq!(scalars[0], ark_bn254::Fr::one());

        // (2^247).to_le_bytes() -> vec![Fr(31), Fr(2^247)]
        let pow_2_to_247 = b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80";
        let scalars = pack_bytes_to_scalars(pow_2_to_247.as_slice()).unwrap();
        assert_eq!(scalars.len(), 1);
        let pow_2_to_247_le_bytes = BigUint::from(2u8).pow(247).to_bytes_le();
        assert_eq!(
            scalars[0],
            ark_bn254::Fr::from_le_bytes_mod_order(pow_2_to_247_le_bytes.as_slice())
        );

        // (2^248).to_le_bytes() -> vec![Fr(32), Fr(2^248)]
        let pow_2_to_248 = b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";
        let scalars = pack_bytes_to_scalars(pow_2_to_248.as_slice()).unwrap();
        assert_eq!(scalars.len(), 2);
        assert_eq!(scalars[0], ark_bn254::Fr::zero());
        assert_eq!(scalars[1], ark_bn254::Fr::one());

        // Trying to pack the max # of bytes should NOT fail
        let full_bytes = (0..MAX_NUM_INPUT_BYTES).map(|_| 0xFF).collect::<Vec<u8>>();
        let scalars = pack_bytes_to_scalars(full_bytes.as_slice()).unwrap();
        assert_eq!(scalars.len(), MAX_NUM_INPUT_SCALARS);

        let mut expected_bytes = (0..BYTES_PACKED_PER_SCALAR)
            .map(|_| 0xFF)
            .collect::<Vec<u8>>();
        expected_bytes.push(0x00); // last 32nd byte is zero
        for scalar in scalars.iter().take(MAX_NUM_INPUT_SCALARS).skip(1) {
            assert_eq!(scalar.into_bigint().to_bytes_le(), expected_bytes)
        }

        // Trying to pack 1 more byte than allowed should fail
        let too_many_bytes = (0..MAX_NUM_INPUT_BYTES + 1)
            .map(|_| 0xFF)
            .collect::<Vec<u8>>();
        let result = pack_bytes_to_scalars(too_many_bytes.as_slice());
        assert!(result.is_err())
    }
}
