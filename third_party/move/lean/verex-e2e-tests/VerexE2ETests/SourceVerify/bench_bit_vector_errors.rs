// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// False twins of `bench/rust/bit_vector`: each claim below fails.

pub struct BitVector {
    bits: Vec<bool>,
}

impl BitVector {
    pub fn new_unbounded(length: usize) -> BitVector {
        if length == 0 || length >= 1024 {
            std::process::abort();
        }
        BitVector { bits: vec![false; length] }
    }

    pub fn new_longer(length: usize) -> BitVector {
        if length == 0 || length >= 1024 {
            std::process::abort();
        }
        BitVector { bits: vec![false; length] }
    }

    pub fn set(&mut self, index: usize) {
        self.bits[index] = true;
    }

    pub fn unset_other(&mut self, index: usize) {
        self.bits[index] = false;
    }

    pub fn shift_left(&mut self, amount: usize) {
        let length = self.bits.len();
        if amount >= length {
            self.bits.fill(false);
        } else {
            for i in amount..length {
                self.bits[i - amount] = self.bits[i];
            }
            for i in length - amount..length {
                self.bits[i] = false;
            }
        }
    }

    pub fn longest_set_sequence_starting_at(&self, start: usize) -> usize {
        if start >= self.bits.len() {
            std::process::abort();
        }
        let mut index = start;
        while index < self.bits.len() && self.bits[index] {
            index += 1;
        }
        index - start
    }
}
