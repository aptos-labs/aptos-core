// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub fn one_positions(bits: &[bool]) -> Vec<usize> {
    bits.iter()
        .enumerate()
        .filter(|(_pos, &val)| val)
        .map(|(pos, _val)| pos)
        .collect()
}

pub fn bits_to_byte(bits: &[bool]) -> u8 {
    let mut ret: u8 = 0;
    for &bit in bits {
        ret = (ret << 1) + (bit as u8)
    }
    ret
}
