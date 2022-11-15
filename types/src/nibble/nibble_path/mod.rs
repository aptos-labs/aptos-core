// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! NibblePath library simplify operations with nibbles in a compact format for modified sparse
//! Merkle tree by providing powerful iterators advancing by either bit or nibble.

#[cfg(test)]
mod nibble_path_test;

use crate::misc::bits_to_byte;
use crate::nibble::NIBBLE_SIZE_IN_BITS;
use crate::{
    nibble::{Nibble, ROOT_NIBBLE_HEIGHT},
    state_store::state_key::StateKey,
};
use aptos_crypto::hash::CryptoHash;
use move_core_types::gas_algebra::Byte;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{collection::vec, prelude::*};
use serde::{Deserialize, Serialize};
use std::{fmt, iter::FromIterator};

/// NibblePath defines a path in Merkle tree in the unit of nibble (4 bits).
#[derive(Clone, Hash, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NibblePath {
    nibbles: Vec<Nibble>,
}

impl Default for NibblePath {
    fn default() -> Self {
        NibblePath { nibbles: vec![] }
    }
}
/// Supports debug format by concatenating nibbles literally. For example, [0x12, 0xa0] with 3
/// nibbles will be printed as "12a".
// impl fmt::Debug for NibblePath {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         self.nibbles().try_for_each(|x| write!(f, "{:0.2x}", x))
//     }
// }

/// Convert a vector of bytes into `NibblePath` using the lower 4 bits of each byte as nibble.
impl FromIterator<Nibble> for NibblePath {
    fn from_iter<I: IntoIterator<Item = Nibble>>(iter: I) -> Self {
        let mut nibble_path = NibblePath::default();
        for nibble in iter {
            nibble_path.push(nibble);
        }
        nibble_path
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for NibblePath {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        arb_nibble_path().boxed()
    }
    type Strategy = BoxedStrategy<Self>;
}

#[cfg(any(test, feature = "fuzzing"))]
prop_compose! {
    fn arb_nibble_path()(
        mut bytes in vec(any::<u8>(), 0..=ROOT_NIBBLE_HEIGHT*NIBBLE_SIZE_IN_BITS/8)
    ) -> NibblePath {
        NibblePath::new_from_bytes(bytes.as_slice(), ROOT_NIBBLE_HEIGHT)
    }
}

#[cfg(any(test, feature = "fuzzing"))]
prop_compose! {
    fn arb_internal_nibble_path()(
        nibble_path in arb_nibble_path().prop_filter(
            "Filter out leaf paths.",
            |p| p.num_nibbles() < ROOT_NIBBLE_HEIGHT,
        )
    ) -> NibblePath {
        nibble_path
    }
}

pub fn byte_to_bits(byte: &u8) -> Vec<bool> {
    (0..8)
        .map(|i| {
            let mask = 1 << (7 - i);
            byte & mask != 0
        })
        .collect()
}

impl NibblePath {
    pub fn new_from_state_key(state_key: &StateKey, num_nibbles: usize) -> Self {
        NibblePath::new_from_bytes(state_key.hash().as_ref(), num_nibbles)
    }

    /// Will panic if not enough bytes are provided to make `required_nibble_count` nibbles.
    pub fn new_from_bytes(bytes: &[u8], required_nibble_count: usize) -> Self {
        assert!(required_nibble_count <= ROOT_NIBBLE_HEIGHT);
        let bits: Vec<bool> = bytes.iter().map(byte_to_bits).flatten().collect();
        Self::new_from_bits(bits.as_slice(), required_nibble_count)
    }

    pub fn new_from_bits(bits: &[bool], required_nibble_count: usize) -> Self {
        assert!(bits.len() >= NIBBLE_SIZE_IN_BITS * required_nibble_count);
        let nibbles = (0..required_nibble_count)
            .map(|i| {
                let start = NIBBLE_SIZE_IN_BITS * i;
                let end = NIBBLE_SIZE_IN_BITS * (i + 1);
                Nibble::from(&bits[start..end])
            })
            .collect();
        Self { nibbles }
    }
    /// Adds a nibble to the end of the nibble path.
    pub fn push(&mut self, nibble: Nibble) {
        assert!(ROOT_NIBBLE_HEIGHT > self.num_nibbles());
        self.nibbles.push(nibble);
    }

    /// Pops a nibble from the end of the nibble path.
    pub fn pop(&mut self) -> Option<Nibble> {
        self.nibbles.pop()
    }

    /// Returns the last nibble.
    pub fn last(&self) -> Option<Nibble> {
        self.nibbles.last().map(|n| *n)
    }

    /// Get the i-th bit.
    fn get_bit(&self, i: usize) -> bool {
        let nid = i / NIBBLE_SIZE_IN_BITS;
        let bid = i % NIBBLE_SIZE_IN_BITS;
        self.nibbles[nid].get_bit(bid)
    }

    /// Get the i-th nibble.
    pub fn get_nibble(&self, i: usize) -> Nibble {
        self.nibbles[i]
    }

    /// Get a bit iterator iterates over the whole nibble path.
    pub fn bits(&self) -> BitIterator {
        BitIterator {
            nibble_path: self,
            pos: (0..NIBBLE_SIZE_IN_BITS * self.num_nibbles()),
        }
    }

    /// Get a nibble iterator iterates over the whole nibble path.
    pub fn nibbles(&self) -> NibbleIterator {
        NibbleIterator::new(self, 0, self.num_nibbles())
    }

    /// Get the total number of nibbles stored.
    pub fn num_nibbles(&self) -> usize {
        self.nibbles.len()
    }

    ///  Returns `true` if the nibbles contains no elements.
    pub fn is_empty(&self) -> bool {
        self.num_nibbles() == 0
    }

    /// Get the underlying bytes storing nibbles.
    pub fn bytes(&self) -> ByteIterator {
        ByteIterator {
            bit_iterator: self.bits(),
        }
    }

    pub fn truncate(&mut self, len: usize) {
        self.nibbles.truncate(len)
    }
}

pub trait Peekable: Iterator {
    /// Returns the `next()` value without advancing the iterator.
    fn peek(&self) -> Option<Self::Item>;
}

/// BitIterator iterates a nibble path by bit.
pub struct BitIterator<'a> {
    nibble_path: &'a NibblePath,
    pos: std::ops::Range<usize>,
}

impl<'a> Peekable for BitIterator<'a> {
    /// Returns the `next()` value without advancing the iterator.
    fn peek(&self) -> Option<Self::Item> {
        if self.pos.start < self.pos.end {
            Some(self.nibble_path.get_bit(self.pos.start))
        } else {
            None
        }
    }
}

/// BitIterator spits out a boolean each time. True/false denotes 1/0.
impl<'a> Iterator for BitIterator<'a> {
    type Item = bool;
    fn next(&mut self) -> Option<Self::Item> {
        self.pos.next().map(|i| self.nibble_path.get_bit(i))
    }
}

/// Support iterating bits in reversed order.
impl<'a> DoubleEndedIterator for BitIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.pos.next_back().map(|i| self.nibble_path.get_bit(i))
    }
}

/// NibbleIterator iterates a nibble path by nibble.
#[derive(Debug)]
pub struct NibbleIterator<'a> {
    /// The underlying nibble path that stores the nibbles
    nibble_path: &'a NibblePath,

    /// The current index, `pos.start`, will bump by 1 after calling `next()` until `pos.start ==
    /// pos.end`.
    pos: std::ops::Range<usize>,

    /// The start index of the iterator. At the beginning, `pos.start == start`. [start, pos.end)
    /// defines the range of `nibble_path` this iterator iterates over. `nibble_path` refers to
    /// the entire underlying buffer but the range may only be partial.
    start: usize,
    // invariant self.start <= self.pos.start;
    // invariant self.pos.start <= self.pos.end;
    // invariant self.pos.end <= ROOT_NIBBLE_HEIGHT;
}

/// NibbleIterator spits out a byte each time. Each byte must be in range [0, 16).
impl<'a> Iterator for NibbleIterator<'a> {
    type Item = Nibble;
    fn next(&mut self) -> Option<Self::Item> {
        self.pos.next().map(|i| self.nibble_path.get_nibble(i))
    }
}

impl<'a> Peekable for NibbleIterator<'a> {
    /// Returns the `next()` value without advancing the iterator.
    fn peek(&self) -> Option<Self::Item> {
        if self.pos.start < self.pos.end {
            Some(self.nibble_path.get_nibble(self.pos.start))
        } else {
            None
        }
    }
}

impl<'a> NibbleIterator<'a> {
    fn new(nibble_path: &'a NibblePath, start: usize, end: usize) -> Self {
        assert!(start <= end);
        assert!(start <= ROOT_NIBBLE_HEIGHT);
        assert!(end <= ROOT_NIBBLE_HEIGHT);
        Self {
            nibble_path,
            pos: (start..end),
            start,
        }
    }

    /// Returns a nibble iterator that iterates all visited nibbles.
    pub fn visited_nibbles(&self) -> NibbleIterator<'a> {
        Self::new(self.nibble_path, self.start, self.pos.start)
    }

    /// Returns a nibble iterator that iterates all remaining nibbles.
    pub fn remaining_nibbles(&self) -> NibbleIterator<'a> {
        Self::new(self.nibble_path, self.pos.start, self.pos.end)
    }

    /// Turn it into a `BitIterator`.
    pub fn bits(&self) -> BitIterator<'a> {
        BitIterator {
            nibble_path: self.nibble_path,
            pos: (self.pos.start * 4..self.pos.end * 4),
        }
    }

    /// Cut and return the range of the underlying `nibble_path` that this iterator is iterating
    /// over as a new `NibblePath`
    pub fn get_nibble_path(&self) -> NibblePath {
        self.visited_nibbles()
            .chain(self.remaining_nibbles())
            .collect()
    }

    /// Get the number of nibbles that this iterator covers.
    pub fn num_nibbles(&self) -> usize {
        assert!(self.start <= self.pos.end); // invariant
        self.pos.end - self.start
    }

    /// Return `true` if the iteration is over.
    pub fn is_finished(&self) -> bool {
        self.peek().is_none()
    }
}

/// Advance both iterators if their next nibbles are the same until either reaches the end or
/// the find a mismatch. Return the number of matched nibbles.
pub fn skip_common_prefix<I1, I2>(x: &mut I1, y: &mut I2) -> usize
where
    I1: Iterator + Peekable,
    I2: Iterator + Peekable,
    <I1 as Iterator>::Item: std::cmp::PartialEq<<I2 as Iterator>::Item>,
{
    let mut count = 0;
    loop {
        let x_peek = x.peek();
        let y_peek = y.peek();
        if x_peek.is_none()
            || y_peek.is_none()
            || x_peek.expect("cannot be none") != y_peek.expect("cannot be none")
        {
            break;
        }
        count += 1;
        x.next();
        y.next();
    }
    count
}

pub struct ByteIterator<'a> {
    bit_iterator: BitIterator<'a>,
}

impl<'a> Iterator for ByteIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let mut bits: Vec<bool> = Vec::with_capacity(8);

        for _i in 0..8 {
            match self.bit_iterator.next() {
                None => break,
                Some(b) => bits.push(b),
            }
        }

        if bits.len() == 0 {
            None
        } else {
            let paddings = vec![false; 8 - bits.len()];
            bits.extend(paddings.iter());
            Some(bits_to_byte(bits.as_slice()))
        }
    }
}
