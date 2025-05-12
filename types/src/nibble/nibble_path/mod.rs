// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! NibblePath library simplify operations with nibbles in a compact format for modified sparse
//! Merkle tree by providing powerful iterators advancing by either bit or nibble.

#[cfg(test)]
mod nibble_path_test;

use crate::{
    nibble::{Nibble, ROOT_NIBBLE_HEIGHT},
    state_store::state_key::StateKey,
};
use aptos_crypto::hash::CryptoHash;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{collection::vec, prelude::*};
use serde::{Deserialize, Serialize};
use std::{fmt, iter::FromIterator};

/// NibblePath defines a path in Merkle tree in the unit of nibble (4 bits).
#[derive(Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NibblePath {
    /// Indicates the total number of nibbles in bytes. Either `bytes.len() * 2 - 1` or
    /// `bytes.len() * 2`.
    // Guarantees intended ordering based on the top-to-bottom declaration order of the struct's
    // members.
    num_nibbles: usize,
    /// The underlying bytes that stores the path, 2 nibbles per byte. If the number of nibbles is
    /// odd, the second half of the last byte must be 0.
    bytes: Vec<u8>,
    // invariant num_nibbles <= ROOT_NIBBLE_HEIGHT
}

/// Supports debug format by concatenating nibbles literally. For example, [0x12, 0xa0] with 3
/// nibbles will be printed as "12a".
impl fmt::Debug for NibblePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.nibbles().try_for_each(|x| write!(f, "{:x}", x))
    }
}

/// Convert a vector of bytes into `NibblePath` using the lower 4 bits of each byte as nibble.
impl FromIterator<Nibble> for NibblePath {
    fn from_iter<I: IntoIterator<Item = Nibble>>(iter: I) -> Self {
        let mut nibble_path = NibblePath::new_even(vec![]);
        for nibble in iter {
            nibble_path.push(nibble);
        }
        nibble_path
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for NibblePath {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        arb_nibble_path().boxed()
    }
}

#[cfg(any(test, feature = "fuzzing"))]
prop_compose! {
    fn arb_nibble_path()(
        mut bytes in vec(any::<u8>(), 0..=ROOT_NIBBLE_HEIGHT/2),
        is_odd in any::<bool>()
    ) -> NibblePath {
        if let Some(last_byte) = bytes.last_mut() {
            if is_odd {
                *last_byte &= 0xf0;
                return NibblePath::new_odd(bytes);
            }
        }
        NibblePath::new_even(bytes)
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

impl NibblePath {
    /// Creates a new `NibblePath` from a vector of bytes assuming each byte has 2 nibbles.
    pub fn new_even(bytes: Vec<u8>) -> Self {
        assert!(bytes.len() <= ROOT_NIBBLE_HEIGHT / 2);
        let num_nibbles = bytes.len() * 2;
        NibblePath { num_nibbles, bytes }
    }

    /// Similar to `new()` but asserts that the bytes have one less nibble.
    pub fn new_odd(bytes: Vec<u8>) -> Self {
        assert!(bytes.len() <= ROOT_NIBBLE_HEIGHT / 2);
        assert_eq!(
            bytes.last().expect("Should have odd number of nibbles.") & 0x0F,
            0,
            "Last nibble must be 0."
        );
        let num_nibbles = bytes.len() * 2 - 1;
        NibblePath { num_nibbles, bytes }
    }

    pub fn new_from_state_key(state_key: &StateKey, num_nibbles: usize) -> Self {
        NibblePath::new_from_byte_array(state_key.hash().as_ref(), num_nibbles)
    }

    fn new_from_byte_array(bytes: &[u8], num_nibbles: usize) -> Self {
        assert!(num_nibbles <= ROOT_NIBBLE_HEIGHT);
        if num_nibbles % 2 == 1 {
            // Rounded up number of bytes to be considered
            let num_bytes = (num_nibbles + 1) / 2;
            assert!(bytes.len() >= num_bytes);
            let mut nibble_bytes = bytes[..num_bytes].to_vec();
            // If number of nibbles is odd, make sure to pad the last nibble with 0s.
            let last_byte_padded = bytes[num_bytes - 1] & 0xF0;
            nibble_bytes[num_bytes - 1] = last_byte_padded;
            NibblePath::new_odd(nibble_bytes)
        } else {
            assert!(bytes.len() >= num_nibbles / 2);
            NibblePath::new_even(bytes[..num_nibbles / 2].to_vec())
        }
    }

    /// Adds a nibble to the end of the nibble path.
    pub fn push(&mut self, nibble: Nibble) {
        assert!(ROOT_NIBBLE_HEIGHT > self.num_nibbles);
        if self.num_nibbles % 2 == 0 {
            self.bytes.push(u8::from(nibble) << 4);
        } else {
            self.bytes[self.num_nibbles / 2] |= u8::from(nibble);
        }
        self.num_nibbles += 1;
    }

    /// Pops a nibble from the end of the nibble path.
    pub fn pop(&mut self) -> Option<Nibble> {
        let poped_nibble = if self.num_nibbles % 2 == 0 {
            self.bytes.last_mut().map(|last_byte| {
                let nibble = *last_byte & 0x0F;
                *last_byte &= 0xF0;
                Nibble::from(nibble)
            })
        } else {
            self.bytes.pop().map(|byte| Nibble::from(byte >> 4))
        };
        if poped_nibble.is_some() {
            self.num_nibbles -= 1;
        }
        poped_nibble
    }

    /// Returns the last nibble.
    pub fn last(&self) -> Option<Nibble> {
        let last_byte_option = self.bytes.last();
        if self.num_nibbles % 2 == 0 {
            last_byte_option.map(|last_byte| Nibble::from(*last_byte & 0x0F))
        } else {
            let last_byte = last_byte_option.expect("Last byte must exist if num_nibbles is odd.");
            Some(Nibble::from(*last_byte >> 4))
        }
    }

    /// Get the i-th bit.
    fn get_bit(&self, i: usize) -> bool {
        assert!(i < self.num_nibbles * 4);
        let pos = i / 8;
        let bit = 7 - i % 8;
        ((self.bytes[pos] >> bit) & 1) != 0
    }

    /// Get the i-th nibble.
    pub fn get_nibble(&self, i: usize) -> Nibble {
        assert!(i < self.num_nibbles);
        Nibble::from((self.bytes[i / 2] >> (if i % 2 == 1 { 0 } else { 4 })) & 0xF)
    }

    /// Get a bit iterator iterates over the whole nibble path.
    pub fn bits(&self) -> BitIterator {
        BitIterator {
            nibble_path: self,
            pos: (0..self.num_nibbles * 4),
        }
    }

    /// Get a nibble iterator iterates over the whole nibble path.
    pub fn nibbles(&self) -> NibbleIterator {
        NibbleIterator::new(self, 0, self.num_nibbles)
    }

    /// Get the total number of nibbles stored.
    pub fn num_nibbles(&self) -> usize {
        self.num_nibbles
    }

    ///  Returns `true` if the nibbles contains no elements.
    pub fn is_empty(&self) -> bool {
        self.num_nibbles() == 0
    }

    /// Get the underlying bytes storing nibbles.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn truncate(&mut self, len: usize) {
        assert!(len <= self.num_nibbles);
        self.num_nibbles = len;
        self.bytes.truncate((len + 1) / 2);
        if len % 2 != 0 {
            *self.bytes.last_mut().expect("must exist.") &= 0xF0;
        }
    }

    // Returns the shard_id of the NibblePath, or None if it is root.
    pub fn get_shard_id(&self) -> Option<u8> {
        if self.num_nibbles() > 0 {
            Some(u8::from(self.get_nibble(0)))
        } else {
            None
        }
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

impl Peekable for BitIterator<'_> {
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
impl Iterator for BitIterator<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos.next().map(|i| self.nibble_path.get_bit(i))
    }
}

/// Support iterating bits in reversed order.
impl DoubleEndedIterator for BitIterator<'_> {
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
impl Iterator for NibbleIterator<'_> {
    type Item = Nibble;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos.next().map(|i| self.nibble_path.get_nibble(i))
    }
}

impl Peekable for NibbleIterator<'_> {
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
