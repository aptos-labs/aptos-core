// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use super::bits::Bits;
use tracing::debug;

pub fn jwt_bit_len(jwt: &[u8]) -> usize {
    jwt.len() * 8
}

/// input: jwt as base64 without padding.
/// output: length of bit representation of jwt, encoded in big-endian as 8 bits.
pub fn jwt_bit_len_binary(jwt_unsigned: &[u8]) -> Bits {
    let L = jwt_bit_len(jwt_unsigned);

    Bits::raw(&format!("{L:064b}"))
}

/// input: jwt as base64 without padding.
/// output: bit representation of sha padding
pub fn compute_sha_padding(jwt_unsigned: &[u8]) -> Bits {
    let mut padding_bits = Bits::new();
    let L = jwt_bit_len(jwt_unsigned);
    // Following the spec linked here:
    //https://www.rfc-editor.org/rfc/rfc4634.html#section-4.1
    // Step 4.1.a: add bit '1'
    padding_bits += Bits::raw("1");
    // Step 4.1.b Append K '0' bits where K is the smallest non-negative integer solution to L+1+K = 448 mod 512, and L is the length of the message in bits
    // i.e., K is smallest non-neg integer such that L + K + 1 + 64 == 0 mod 512
    // we never expect this to cause an error, so unwrapping here is ok
    // There was a bug here, which is why we are logging so much
    let K_before_mod = 448 - (L as i64) - 1;
    debug!("Computing sha padding: K_before_mod={}", K_before_mod);
    let K_i64 = K_before_mod.rem_euclid(512);
    debug!("Computing sha padding: K_i64={}", K_i64);
    let K_usize = usize::try_from(K_i64).unwrap();
    debug!("Computing sha padding: K_usize={}", K_usize);
    padding_bits += Bits::raw(&("0".repeat(K_usize)));
    // 4.1.c Append L in binary form (big-endian) as 64 bits
    padding_bits += jwt_bit_len_binary(jwt_unsigned);

    padding_bits
}

pub fn compute_sha_padding_without_len(jwt_unsigned: &[u8]) -> Bits {
    let mut padding_bits = Bits::new();
    let L = jwt_bit_len(jwt_unsigned);
    // Following the spec linked here:
    //https://www.rfc-editor.org/rfc/rfc4634.html#section-4.1
    // Step 4.1.a: add bit '1'
    padding_bits += Bits::raw("1");
    // Step 4.1.b Append K '0' bits where K is the smallest non-negative integer solution to L+1+K = 448 mod 512, and L is the length of the message in bits
    // i.e., K is smallest non-neg integer such that L + K + 1 + 64 == 0 mod 512
    // we never expect this to cause an error, so unwrapping here is ok
    // There was a bug here, which is why we are logging so much
    let K_before_mod = 448 - (L as i64) - 1;
    debug!("Computing sha padding: K_before_mod={}", K_before_mod);
    let K_i64 = K_before_mod.rem_euclid(512);
    debug!("Computing sha padding: K_i64={}", K_i64);
    let K_usize = usize::try_from(K_i64).unwrap();
    debug!("Computing sha padding: K_usize={}", K_usize);
    padding_bits += Bits::raw(&("0".repeat(K_usize)));
    // Skip 4.1.c
    padding_bits
}

pub fn with_sha_padding_bytes(jwt_unsigned: &[u8]) -> Vec<u8> {
    (Bits::bit_representation_of_bytes(jwt_unsigned) + compute_sha_padding(jwt_unsigned))
        .as_bytes()
        .expect("Should have length a multiple of 8")
}
