// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! `std::bit_vector` in Rust, over `Vec<bool>`.

pub struct BitVector {
    bits: Vec<bool>,
}

impl BitVector {
    pub fn new(length: usize) -> BitVector {
        // Idiomatic: `assert!`, and `MAX_SIZE` as a `const`; the exporter rejects both.
        if length == 0 || length >= 1024 {
            std::process::abort();
        }
        BitVector { bits: vec![false; length] }
    }

    pub fn set(&mut self, index: usize) {
        self.bits[index] = true;
    }

    pub fn unset(&mut self, index: usize) {
        self.bits[index] = false;
    }

    pub fn is_index_set(&self, index: usize) -> bool {
        self.bits[index]
    }

    pub fn length(&self) -> usize {
        self.bits.len()
    }

    pub fn shift_left(&mut self, amount: usize) {
        let length = self.bits.len();
        // The index loops stand for `copy_within`, `fill` of a subslice, and `iter_mut`.
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
        // Idiomatic: `assert!`.
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
