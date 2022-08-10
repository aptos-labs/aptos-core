// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This library defines a BitVec struct that represents a bit vector.

#[cfg(any(test, feature = "fuzzing"))]
use proptest::{
    arbitrary::{any, Arbitrary, StrategyFor},
    collection::{vec, VecStrategy},
    strategy::{Map, Strategy},
};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{
    iter::FromIterator,
    ops::{BitAnd, BitOr},
};

// Every u8 is used as a bucket of 8 bits. Total max buckets = 65536 / 8 = 8196.
const BUCKET_SIZE: usize = 8;
const MAX_BUCKETS: usize = 8192;

/// BitVec represents a bit vector that supports 4 operations:
///
/// 1. Marking a position as set.
/// 2. Checking if a position is set.
/// 3. Count set bits.
/// 4. Get the index of the last set bit.
///
/// Internally, it stores a vector of u8's (as Vec<u8>).
///
/// * The first 8 positions of the bit vector are encoded in the first element of the vector, the
///   next 8 are encoded in the second element, and so on.
/// * Bits are read from left to right. For instance, in the following bitvec
///   [0b0001_0000, 0b0000_0000, 0b0000_0000, 0b0000_0001], the 3rd and 31st positions are set.
/// * Each bit of a u8 is set to 1 if the position is set and to 0 if it's not.
/// * We only allow setting positions upto u16::MAX. As a result, the size of the inner vector is
///   limited to 8192 (= 65536 / 8).
/// * Once a bit has been set, it cannot be unset. As a result, the inner vector cannot shrink.
/// * The positions can be set in any order.
/// * A position can set more than once -- it remains set after the first time.
///
/// # Examples:
/// ```
/// use aptos_bitvec::BitVec;
/// use std::ops::BitAnd;
///
/// let mut bv = BitVec::default();
/// bv.set(2);
/// bv.set(5);
/// assert!(bv.is_set(2));
/// assert!(bv.is_set(5));
/// assert_eq!(false, bv.is_set(0));
/// assert_eq!(bv.count_ones(), 2);
/// assert_eq!(bv.last_set_bit(), Some(5));
///
/// // A bitwise AND of BitVec can be performed by using the `&` operator.
/// let mut bv1 = BitVec::default();
/// bv1.set(2);
/// bv1.set(3);
/// let mut bv2 = BitVec::default();
/// bv2.set(2);
/// let intersection = bv1.bitand(&bv2);
/// assert!(intersection.is_set(2));
/// assert_eq!(false, intersection.is_set(3));
/// ```
#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize)]
pub struct BitVec {
    #[serde(with = "serde_bytes")]
    inner: Vec<u8>,
}

impl BitVec {
    fn with_capacity(num_buckets: usize) -> Self {
        Self {
            inner: Vec::with_capacity(num_buckets),
        }
    }

    /// Initialize with buckets that can fit in num_bits.
    pub fn with_num_bits(num_bits: u16) -> Self {
        Self {
            inner: vec![0; Self::required_buckets(num_bits)],
        }
    }

    /// Sets the bit at position @pos.
    pub fn set(&mut self, pos: u16) {
        // This is optimised to: let bucket = pos >> 3;
        let bucket: usize = pos as usize / BUCKET_SIZE;
        if self.inner.len() <= bucket {
            self.inner.resize(bucket + 1, 0);
        }
        // This is optimized to: let bucket_pos = pos | 0x07;
        let bucket_pos = pos as usize - (bucket * BUCKET_SIZE);
        self.inner[bucket] |= 0b1000_0000 >> bucket_pos as u8;
    }

    /// Checks if the bit at position @pos is set.
    #[inline]
    pub fn is_set(&self, pos: u16) -> bool {
        // This is optimised to: let bucket = pos >> 3;
        let bucket: usize = pos as usize / BUCKET_SIZE;
        if self.inner.len() <= bucket {
            return false;
        }
        // This is optimized to: let bucket_pos = pos | 0x07;
        let bucket_pos = pos as usize - (bucket * BUCKET_SIZE);
        (self.inner[bucket] & (0b1000_0000 >> bucket_pos as u8)) != 0
    }

    /// Return true if the BitVec is all zeros.
    pub fn all_zeros(&self) -> bool {
        self.inner.iter().all(|byte| *byte == 0)
    }

    /// Returns the number of set bits.
    pub fn count_ones(&self) -> u32 {
        self.inner.iter().map(|a| a.count_ones()).sum()
    }

    /// Returns the index of the last set bit.
    pub fn last_set_bit(&self) -> Option<u16> {
        self.inner
            .iter()
            .rev()
            .enumerate()
            .find(|(_, byte)| byte != &&0u8)
            .map(|(i, byte)| {
                (8 * (self.inner.len() - i) - byte.trailing_zeros() as usize - 1) as u16
            })
    }

    /// Return an `Iterator` over all '1' bit indexes.
    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.inner.len() * BUCKET_SIZE).filter(move |idx| self.is_set(*idx as u16))
    }

    /// Return the number of buckets.
    pub fn num_buckets(&self) -> usize {
        self.inner.len()
    }

    /// Number of buckets require for num_bits.
    pub fn required_buckets(num_bits: u16) -> usize {
        num_bits
            .checked_sub(1)
            .map_or(0, |pos| pos as usize / BUCKET_SIZE + 1)
    }
}

impl BitAnd for &BitVec {
    type Output = BitVec;

    /// Returns a new BitVec that is a bitwise AND of two BitVecs.
    fn bitand(self, other: Self) -> Self::Output {
        let len = std::cmp::min(self.inner.len(), other.inner.len());
        let mut ret = BitVec::with_capacity(len);
        for i in 0..len {
            ret.inner.push(self.inner[i] & other.inner[i]);
        }
        ret
    }
}

impl BitOr for &BitVec {
    type Output = BitVec;

    /// Returns a new BitVec that is a bitwise OR of two BitVecs.
    fn bitor(self, other: Self) -> Self::Output {
        let len = std::cmp::max(self.inner.len(), other.inner.len());
        let mut ret = BitVec::with_capacity(len);
        for i in 0..len {
            let a = self.inner.get(i).copied().unwrap_or(0);
            let b = other.inner.get(i).copied().unwrap_or(0);
            ret.inner.push(a | b);
        }
        ret
    }
}

impl FromIterator<u8> for BitVec {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut bitvec = Self::default();
        for bit in iter {
            bitvec.set(bit as u16);
        }
        bitvec
    }
}

impl From<Vec<u8>> for BitVec {
    fn from(raw_bytes: Vec<u8>) -> Self {
        assert!(raw_bytes.len() <= MAX_BUCKETS);
        Self { inner: raw_bytes }
    }
}

impl From<BitVec> for Vec<u8> {
    fn from(bitvec: BitVec) -> Self {
        bitvec.inner
    }
}

impl From<Vec<bool>> for BitVec {
    fn from(bits: Vec<bool>) -> Self {
        assert!(bits.len() <= MAX_BUCKETS * BUCKET_SIZE);
        let mut bitvec = Self::with_num_bits(bits.len() as u16);
        for (index, b) in bits.iter().enumerate() {
            if *b {
                bitvec.set(index as u16);
            }
        }
        bitvec
    }
}

impl<'de> Deserialize<'de> for BitVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "BitVec")]
        struct RawData {
            #[serde(with = "serde_bytes")]
            inner: Vec<u8>,
        }
        let v = RawData::deserialize(deserializer)?.inner;
        if v.len() > MAX_BUCKETS {
            return Err(D::Error::custom(format!("BitVec too long: {}", v.len())));
        }
        Ok(BitVec { inner: v })
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for BitVec {
    type Parameters = ();
    type Strategy = Map<VecStrategy<StrategyFor<u8>>, fn(Vec<u8>) -> BitVec>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        vec(any::<u8>(), 0..=MAX_BUCKETS).prop_map(|inner| BitVec { inner })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::proptest;

    #[test]
    fn test_count_ones() {
        let p0 = BitVec::default();
        assert_eq!(p0.count_ones(), 0);
        // 7 = b'0000111' and 240 = b'00001111'
        let p1 = BitVec {
            inner: vec![7u8, 15u8],
        };
        assert_eq!(p1.count_ones(), 7);

        let p2 = BitVec {
            inner: vec![7u8; MAX_BUCKETS],
        };
        assert_eq!(p2.count_ones(), 3 * MAX_BUCKETS as u32);

        // 255 = b'11111111'
        let p3 = BitVec {
            inner: vec![255u8; MAX_BUCKETS],
        };
        assert_eq!(p3.count_ones(), 8 * MAX_BUCKETS as u32);

        // 0 = b'00000000'
        let p4 = BitVec {
            inner: vec![0u8; MAX_BUCKETS],
        };
        assert_eq!(p4.count_ones(), 0);
    }

    #[test]
    fn test_last_set_bit() {
        let p0 = BitVec::default();
        assert_eq!(p0.last_set_bit(), None);
        // 224 = b'11100000'
        let p1 = BitVec { inner: vec![224u8] };
        assert_eq!(p1.inner.len(), 1);
        assert_eq!(p1.last_set_bit(), Some(2));

        // 128 = 0b1000_0000
        let p2 = BitVec {
            inner: vec![7u8, 128u8],
        };
        assert_eq!(p2.inner.len(), 2);
        assert_eq!(p2.last_set_bit(), Some(8));

        let p3 = BitVec {
            inner: vec![255u8; MAX_BUCKETS],
        };
        assert_eq!(p3.inner.len(), MAX_BUCKETS);
        assert_eq!(p3.last_set_bit(), Some(65535));

        let p4 = BitVec {
            inner: vec![0u8; MAX_BUCKETS],
        };
        assert_eq!(p4.last_set_bit(), None);

        // An extra test to ensure left to right encoding.
        let mut p5 = BitVec {
            inner: vec![0b0000_0001, 0b0100_0000],
        };
        assert_eq!(p5.last_set_bit(), Some(9));
        assert!(p5.is_set(7));
        assert!(p5.is_set(9));
        assert!(!p5.is_set(0));

        p5.set(10);
        assert!(p5.is_set(10));
        assert_eq!(p5.last_set_bit(), Some(10));
        assert_eq!(p5.inner, vec![0b0000_0001, 0b0110_0000]);

        let p6 = BitVec {
            inner: vec![0b1000_0000],
        };
        assert_eq!(p6.inner.len(), 1);
        assert_eq!(p6.last_set_bit(), Some(0));
    }

    #[test]
    fn test_empty() {
        let p = BitVec::default();
        for i in 0..=u16::MAX {
            assert!(!p.is_set(i));
        }
    }

    #[test]
    fn test_extremes() {
        let mut p = BitVec::default();
        p.set(u16::MAX);
        p.set(0);
        assert!(p.is_set(u16::MAX));
        assert!(p.is_set(0));
        for i in 1..u16::MAX {
            assert!(!p.is_set(i));
        }
        assert_eq!(
            vec![0, u16::MAX as usize],
            p.iter_ones().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_conversion() {
        let bitmaps = vec![
            false, true, true, false, false, true, true, false, true, true, true,
        ];
        let bitvec = BitVec::from(bitmaps.clone());
        for (index, is_set) in bitmaps.into_iter().enumerate() {
            assert_eq!(bitvec.is_set(index as u16), is_set);
        }
    }

    #[test]
    fn test_deserialization() {
        let raw = vec![0u8; 9000];
        let bytes = bcs::to_bytes(&raw).unwrap();
        assert!(bcs::from_bytes::<Vec<u8>>(&bytes).is_ok());
        // 9000 > MAX_BUCKET:
        assert!(bcs::from_bytes::<BitVec>(&bytes).is_err());
        let mut bytes = [0u8; 33];
        bytes[0] = 32;
        let bv = BitVec {
            inner: Vec::from([0u8; 32].as_ref()),
        };
        assert_eq!(Ok(bv), bcs::from_bytes::<BitVec>(&bytes));
    }

    // Test for bitwise AND operation on 2 bitvecs.
    proptest! {
        #[test]
        fn test_and(bv1 in any::<BitVec>(), bv2 in any::<BitVec>()) {
            let intersection = bv1.bitand(&bv2);

            assert!(intersection.count_ones() <= bv1.count_ones());
            assert!(intersection.count_ones() <= bv2.count_ones());

            for i in 0..=u16::MAX {
                if bv1.is_set(i) && bv2.is_set(i) {
                    assert!(intersection.is_set(i));
                } else {
                    assert!(!intersection.is_set(i));
                }
            }
        }

        #[test]
        fn test_or(bv1 in any::<BitVec>(), bv2 in any::<BitVec>()) {
            let union = bv1.bitor(&bv2);

            assert!(union.count_ones() >= bv1.count_ones());
            assert!(union.count_ones() >= bv2.count_ones());

            for i in 0..=u16::MAX {
                if bv1.is_set(i) || bv2.is_set(i) {
                    assert!(union.is_set(i));
                } else {
                    assert!(!union.is_set(i));
                }
            }
        }

        #[test]
        fn test_iter_ones(bv1 in any::<BitVec>()) {
            assert_eq!(bv1.iter_ones().count(), bv1.count_ones() as usize);
        }

        #[test]
        fn test_serde_roundtrip(bits in vec(any::<bool>(), 0..u16::MAX as usize)) {
            let bitvec = BitVec::from(bits);
            let bytes = serde_json::to_vec(&bitvec).unwrap();
            let back = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(bitvec, back);
        }

    }
}
