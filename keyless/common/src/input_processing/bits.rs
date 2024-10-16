// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use std::ops::{self, Add, AddAssign};

/// Type for bit representation during conversion. Represents bits using strings, for easy
/// manipulation:
/// ```ignore
/// use prover_service::input_processing::bits::Bits;
/// let b = Bits::raw("00001111");
/// assert_eq!(b.as_bytes().unwrap()[0], 15u8);
/// ```
///
/// This struct is mainly used for the sha padding computation.
#[derive(Debug, Eq, PartialEq)]
pub struct Bits {
    pub(crate) b: String,
}

impl Default for Bits {
    fn default() -> Self {
        Self::new()
    }
}

impl Bits {
    pub fn new() -> Self {
        Bits { b: String::new() }
    }

    /// Input: Bits in BIG-ENDIAN order
    /// Output: bytes in BIG_ENDIAN order
    pub fn as_bytes(self) -> Result<Vec<u8>> {
        if self.b.len() % 8 != 0 {
            bail!("Tried to convert bits to bytes, where bit length is not divisible by 8")
        } else {
            let mut bytes = Vec::new();

            for i in 0..(self.b.len() / 8) {
                let idx = i * 8;
                let bits_for_chunk: &str = &self[idx..idx + 8];
                let chunk_byte =
                    u8::from_str_radix(bits_for_chunk, 2).expect("Binary string should parse");

                bytes.push(chunk_byte);
            }

            Ok(bytes)
        }
    }

    pub fn bit_representation_of_str(s: &str) -> Self {
        let mut bits = Bits::new();
        for byte in s.as_bytes() {
            bits.b += &format!("{byte:08b}");
        }
        bits
    }

    pub fn bit_representation_of_bytes(s: &[u8]) -> Self {
        let mut bits = Bits::new();
        for byte in s {
            bits.b += &format!("{byte:08b}");
        }
        bits
    }

    pub fn raw(b: &str) -> Self {
        Bits { b: String::from(b) }
    }
}

impl ops::Index<ops::Range<usize>> for Bits {
    type Output = str;

    fn index(&self, index: ops::Range<usize>) -> &str {
        self.b.index(index)
    }
}

impl AddAssign<Bits> for Bits {
    fn add_assign(&mut self, rhs: Bits) {
        self.b += &rhs.b;
    }
}

impl Add<Bits> for Bits {
    type Output = Bits;

    fn add(self, rhs: Bits) -> Self::Output {
        Bits { b: self.b + &rhs.b }
    }
}

impl From<Bits> for String {
    fn from(value: Bits) -> Self {
        value.b
    }
}
