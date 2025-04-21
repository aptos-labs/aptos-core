// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::hash::*;
use bitvec::prelude::*;
use proptest::{collection::vec, prelude::*};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::Serialize;
use std::str::FromStr;

#[derive(Serialize)]
struct Foo(u32);

#[test]
fn test_default_hasher() {
    assert_eq!(
        Foo(3).test_only_hash(),
        HashValue::from_iter_sha3(vec![bcs::to_bytes(&Foo(3)).unwrap().as_slice()]),
    );
    assert_eq!(
        format!("{:x}", b"hello".test_only_hash()),
        "3338be694f50c5f338814986cdf0686453a888b84f424d792af4b9202398f392",
    );
    assert_eq!(
        format!("{:x}", b"world".test_only_hash()),
        "420baf620e3fcd9b3715b42b92506e9304d56e02d3a103499a3a292560cb66b2",
    );
}

#[test]
fn test_primitive_type() {
    let x = 0xF312_u16;
    let mut wtr: Vec<u8> = vec![];
    wtr.extend_from_slice(&x.to_le_bytes());
    assert_eq!(x.test_only_hash(), HashValue::sha3_256_of(&wtr[..]));

    let x = 0xFF00_1234_u32;
    let mut wtr: Vec<u8> = vec![];
    wtr.extend_from_slice(&x.to_le_bytes());
    assert_eq!(x.test_only_hash(), HashValue::sha3_256_of(&wtr[..]));

    let x = 0x89AB_CDEF_0123_4567_u64;
    let mut wtr: Vec<u8> = vec![];
    wtr.extend_from_slice(&x.to_le_bytes());
    assert_eq!(x.test_only_hash(), HashValue::sha3_256_of(&wtr[..]));
}

#[test]
fn test_from_slice() {
    {
        let zero_byte_vec = vec![0; 32];
        assert_eq!(
            HashValue::from_slice(zero_byte_vec).unwrap(),
            HashValue::zero()
        );
    }
    {
        // The length is mismatched.
        let zero_byte_vec = vec![0; 31];
        assert!(HashValue::from_slice(zero_byte_vec).is_err());
    }
    {
        let bytes = [1; 123];
        assert!(HashValue::from_slice(&bytes[..]).is_err());
    }
}

#[test]
fn test_random_with_rng() {
    let mut seed = [0u8; 32];
    seed[..4].copy_from_slice(&[1, 2, 3, 4]);
    let hash1;
    let hash2;
    let hash3;
    {
        let mut rng: StdRng = SeedableRng::from_seed(seed);
        hash1 = HashValue::random_with_rng(&mut rng);
    }
    {
        let mut rng: StdRng = SeedableRng::from_seed(seed);
        hash2 = HashValue::random_with_rng(&mut rng);
    }
    {
        let mut rng: StdRng = SeedableRng::from_seed(seed);
        hash3 = rng.gen();
    }
    assert_eq!(hash1, hash2);
    assert_eq!(hash1, hash3);
}

#[test]
fn test_hash_value_iter_bits() {
    let hash = b"hello".test_only_hash();
    let bits = hash.iter_bits().collect::<Vec<_>>();

    assert_eq!(bits.len(), HashValue::LENGTH_IN_BITS);
    assert!(!bits[0]);
    assert!(!bits[1]);
    assert!(bits[2]);
    assert!(bits[3]);
    assert!(!bits[4]);
    assert!(!bits[5]);
    assert!(bits[6]);
    assert!(bits[7]);
    assert!(bits[248]);
    assert!(!bits[249]);
    assert!(!bits[250]);
    assert!(bits[251]);
    assert!(!bits[252]);
    assert!(!bits[253]);
    assert!(bits[254]);
    assert!(!bits[255]);

    let mut bits_rev = hash.iter_bits().rev().collect::<Vec<_>>();
    bits_rev.reverse();
    assert_eq!(bits, bits_rev);
}

#[test]
fn test_hash_value_iterator_exact_size() {
    let hash = b"hello".test_only_hash();

    let mut iter = hash.iter_bits();
    assert_eq!(iter.len(), HashValue::LENGTH_IN_BITS);
    iter.next();
    assert_eq!(iter.len(), HashValue::LENGTH_IN_BITS - 1);
    iter.next_back();
    assert_eq!(iter.len(), HashValue::LENGTH_IN_BITS - 2);

    let iter_rev = hash.iter_bits().rev();
    assert_eq!(iter_rev.len(), HashValue::LENGTH_IN_BITS);

    let iter_skip = hash.iter_bits().skip(100);
    assert_eq!(iter_skip.len(), HashValue::LENGTH_IN_BITS - 100);
}

#[test]
fn test_fmt_binary() {
    let hash = b"hello".test_only_hash();
    let hash_str = format!("{:b}", hash);
    assert_eq!(hash_str.len(), HashValue::LENGTH_IN_BITS);
    for (bit, chr) in hash.iter_bits().zip(hash_str.chars()) {
        assert_eq!(chr, if bit { '1' } else { '0' });
    }
}

#[test]
fn test_get_nibble() {
    let mut bytes = [0u8; HashValue::LENGTH];
    let mut nibbles = vec![];
    for byte in bytes.iter_mut().take(HashValue::LENGTH) {
        *byte = rand::thread_rng().gen();
        nibbles.push(*byte >> 4);
        nibbles.push(*byte & 0x0F);
    }
    let hash = HashValue::new(bytes);
    for (i, nibble) in nibbles.iter().enumerate().take(HashValue::LENGTH * 2) {
        assert_eq!(hash.nibble(i), *nibble);
    }
}

#[test]
fn test_common_prefix_bits_len() {
    {
        let hash1 = b"hello".test_only_hash();
        let hash2 = b"HELLO".test_only_hash();
        assert_eq!(hash1[0], 0b0011_0011);
        assert_eq!(hash2[0], 0b1011_1000);
        assert_eq!(hash1.common_prefix_bits_len(hash2), 0);
    }
    {
        let hash1 = b"hello".test_only_hash();
        let hash2 = b"world".test_only_hash();
        assert_eq!(hash1[0], 0b0011_0011);
        assert_eq!(hash2[0], 0b0100_0010);
        assert_eq!(hash1.common_prefix_bits_len(hash2), 1);
    }
    {
        let hash1 = b"hello".test_only_hash();
        let hash2 = b"100011001000".test_only_hash();
        assert_eq!(hash1[0], 0b0011_0011);
        assert_eq!(hash2[0], 0b0011_0011);
        assert_eq!(hash1[1], 0b0011_1000);
        assert_eq!(hash2[1], 0b0010_0010);
        assert_eq!(hash1.common_prefix_bits_len(hash2), 11);
    }
    {
        let hash1 = b"hello".test_only_hash();
        let hash2 = b"hello".test_only_hash();
        assert_eq!(
            hash1.common_prefix_bits_len(hash2),
            HashValue::LENGTH_IN_BITS
        );
    }
}

proptest! {
    #[test]
    fn test_hashvalue_to_bits_roundtrip(hash in any::<HashValue>()) {
        let bitvec: BitVec<u8, Msb0>  = hash.iter_bits().collect();
        let bytes: Vec<u8> = bitvec.into();
        let hash2 = HashValue::from_slice(bytes).unwrap();
        prop_assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hashvalue_to_bits_inverse_roundtrip(bits in vec(any::<bool>(), HashValue::LENGTH_IN_BITS)) {
        let bitvec: BitVec<u8, Msb0> = bits.iter().cloned().collect();
        let bytes: Vec<u8> = bitvec.into();
        let hash = HashValue::from_slice(bytes).unwrap();
        let bits2: Vec<bool> = hash.iter_bits().collect();
        prop_assert_eq!(bits, bits2);
    }

    #[test]
    fn test_hashvalue_iter_bits_rev(hash in any::<HashValue>()) {
        let bits1: Vec<bool> = hash.iter_bits().collect();
        let mut bits2: Vec<bool> = hash.iter_bits().rev().collect();
        bits2.reverse();
        prop_assert_eq!(bits1, bits2);
    }

    #[test]
    fn test_hashvalue_to_rev_bits_roundtrip(hash in any::<HashValue>()) {
        let bitvec: BitVec<u8, Lsb0> = hash.iter_bits().rev().collect();
        let mut bytes: Vec<u8> = bitvec.into();
        bytes.reverse();
        let hash2 = HashValue::from_slice(bytes).unwrap();
        prop_assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hashvalue_to_str_roundtrip(hash in any::<HashValue>()) {
        let hash2 = HashValue::from_str(&hash.to_hex()).unwrap();
        prop_assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hashvalue_to_hex_literal(hash in any::<HashValue>()) {
        prop_assert_eq!(format!("0x{}", hash.to_hex()), hash.to_hex_literal());
    }

    #[test]
    fn test_hashvalue_from_bit_iter(hash in any::<HashValue>()) {
        let hash2 = HashValue::from_bit_iter(hash.iter_bits()).unwrap();
        prop_assert_eq!(hash, hash2);

        let bits1 = vec![false; HashValue::LENGTH_IN_BITS - 10];
        prop_assert!(HashValue::from_bit_iter(bits1.into_iter()).is_err());

        let bits2 = vec![false; HashValue::LENGTH_IN_BITS + 10];
        prop_assert!(HashValue::from_bit_iter(bits2.into_iter()).is_err());
    }
}
