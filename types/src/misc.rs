// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub fn one_positions(bits: &[bool]) -> Vec<usize> {
    bits.iter()
        .enumerate()
        .filter(|(_pos, &val)| val)
        .map(|(pos, _val)| pos)
        .collect()
}

/// [T,T,F,F,F,F,F,F] -> 128+64=192
/// [F,T] -> 64
pub fn bits_to_byte(bits: &[bool]) -> u8 {
    assert!(bits.len() <= 8);
    let mut ret = 0;
    let mut pos = 7;
    for &bit in bits {
        ret += (bit as u8) << pos;
        pos -= 1;
    }
    ret
}

/// [T,T,F,F,F,F,F,F,  F,T] -> [128,64]
pub fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    let bit_count = bits.len();
    let byte_count = (bit_count + 7) / 8;
    let mut ret = Vec::with_capacity(byte_count);
    let mut next_bit_pos = 0;
    for _i in 0..(byte_count - 1) {
        ret.push(bits_to_byte(&bits[next_bit_pos..(next_bit_pos + 8)]));
        next_bit_pos += 8;
    }
    ret.push(bits_to_byte(&bits[next_bit_pos..bit_count]));
    ret
}

/// [128,64], bit_count=4 -> [T,T,F,F]
/// [128,64], bit_count=12 -> [T,T,F,F,F,F,F,F, F,T,F,F]
pub fn bytes_to_bits(bytes: &[u8], bit_count: usize) -> Vec<bool> {
    let mut remaining = bit_count;
    let mut ret = Vec::with_capacity(bit_count);
    for &byte in bytes {
        for i in (0..8).rev() {
            let bit = (byte & (1 << i)) != 0;
            ret.push(bit);
            remaining -= 1;
            if remaining == 0 {
                return ret;
            }
        }
    }

    for _i in 0..remaining {
        ret.push(false);
    }

    ret
}
