// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! TBD.
#![allow(clippy::integer_arithmetic)]
use std::cmp::min;

/// TBD
pub trait PippengerFriendlyStructure: Clone {
    /// TBD
    fn add(&self, other: &Self) -> Self;
    /// TBD
    fn add_assign(&mut self, other: &Self);
    /// TBD
    fn double(&self) -> Self;
    /// TBD
    fn double_assign(&mut self);
    /// TBD
    fn neg(&self) -> Self;
    /// TBD
    fn zero() -> Self;
}

/// usize_to_bits(4, 8) == [0,0,1,0,0,0,0,0]
/// usize_to_bits(20, 8) == [0,0,1,0,1,0,0,0]
/// usize_to_bits(1024, 8) == [0,0,0,0,0,0,0,0]
pub fn usize_to_bits(mut v: usize, target_len: usize) -> Vec<bool> {
    let mut ret = vec![false; target_len];
    let mut i = 0;
    while i<target_len && v>0 {
        ret[i] = v%2==1;
        v /= 2;
        i += 1;
    }
    ret
}

/// bits_to_usize(0,0,1,0,0,0,0,0) == 4
/// bits_to_usize(0,0,1,0,1,0,0,0) == 20
/// bits_to_usize(0,0,0,0,0,0,0,0) == 0
pub fn bits_to_usize(bits: &[bool]) -> usize {
    let mut ret = 0;
    for (i,&bit) in bits.iter().enumerate() {
        ret += (bit as usize) << i;
    }
    ret
}



/// Finding the best `window_size`:
///
/// w: window size in bits
/// n: num of elements/scalars.
/// l: scalar size in bits
/// A: add_assign time
/// D: double_assign time
///
/// cost = (n*A + 2*(2^w)*A + w*D + A)* (l/w) = = A*l*(2^(w+1)+n+1)/w + D*l
/// Only need to find w that minimize `(2^(w+1)+n+1)/w`.
///
pub fn find_best_window_size(n: usize) -> usize {
    let precalculated_size_table = vec![0, 0, 8, 32, 96, 256, 640, 1536, 3584, 8192, 18432, 40960, 90112, 196608, 425984, 917504, 1966080, 4194304, 8912896, 18874368, 39845888, 83886080, 176160768, 369098752];
    for (i,&v) in precalculated_size_table.iter().enumerate() {
        if n < v {
            return i;
        }
    }
    precalculated_size_table.len()
}

/// TBD
pub fn probably_pippenger_auto_window_size<S: PippengerFriendlyStructure>(elements: &[S], scalars: &[Vec<bool>]) -> S {
    let num_elements = elements.len();
    let window_size_in_bits = find_best_window_size(num_elements);
    generic_pippenger(elements, scalars, window_size_in_bits)
}

/// TBD
pub fn generic_pippenger<S: PippengerFriendlyStructure>(elements: &[S], scalars: &[Vec<bool>], window_size_in_bits: usize) -> S {
    let num_elements = elements.len();
    let num_scalars = scalars.len();
    assert_eq!(num_elements, num_scalars);
    if num_elements == 0 {
        return S::zero();
    }
    let scalar_size_in_bits = scalars[0].len();
    for i in 1..num_scalars {
        assert_eq!(scalar_size_in_bits, scalars[i].len());
    }
    assert!(window_size_in_bits >= 1);

    let mut ret = S::zero();
    for cur_window_start in (0..scalar_size_in_bits).step_by(window_size_in_bits).rev() {
        let cur_window_end = min(cur_window_start + window_size_in_bits, scalar_size_in_bits);
        let cur_window_size_in_bits = cur_window_end - cur_window_start;
        let cur_num_buckets: usize = 1 << cur_window_size_in_bits;

        // Basically `let buckets = vec![S::zero(); cur_bucket_count];`, but `S` does not need to have `Clone` trait.
        let mut buckets = Vec::with_capacity(cur_num_buckets);
        for _i in 0..cur_num_buckets {
            buckets.push(S::zero())
        }

        for i in 0..num_elements {
            let bucket_id = bits_to_usize(&scalars[i][cur_window_start..cur_window_end]);
            if bucket_id != 0 {
                buckets[bucket_id].add_assign(&elements[i]);
            }
        }

        let mut bucket_sum = S::zero();
        let mut weighted_bucket_sum = S::zero();
        for i in (1..cur_num_buckets).rev() {
            bucket_sum.add_assign(&buckets[i]);
            weighted_bucket_sum.add_assign(&bucket_sum);
        }
        // now weighed_bucket_sim = bucket[1]*1 + bucket[2]*2 + ... + bucket[m-1]*(m-1).

        for _ in 0..cur_window_size_in_bits {
            ret.double_assign();
        }

        ret.add_assign(&weighted_bucket_sum);
    }

    ret
}

/// TBD
pub fn probably_pippenger_signed_digits<S: PippengerFriendlyStructure>(elements: &[S], scalars: &[Vec<bool>], window_size_in_bits: usize) -> S {
    let num_elements = elements.len();
    let num_scalars = scalars.len();
    assert_eq!(num_elements, num_scalars);
    if num_elements == 0 {
        return S::zero();
    }
    let scalar_size_in_bits = scalars[0].len();
    for i in 1..num_scalars {
        assert_eq!(scalar_size_in_bits, scalars[i].len());
    }

    let mut num_windows = (scalar_size_in_bits + window_size_in_bits - 1) / window_size_in_bits;
    assert!(window_size_in_bits >= 1);

    let element_negs: Vec<S> = elements.iter().map(|e| e.neg()).collect();

    let num_buckets: usize = 1 << window_size_in_bits;
    let mut window_vecs: Vec<Vec<usize>> = Vec::with_capacity(num_elements);
    let mut carry = false;
    let mut extra_window = false;
    for i in 0..num_elements {
        let mut window_vec = Vec::with_capacity(num_windows + 1);
        for cur_window_start in (0..scalar_size_in_bits).step_by(window_size_in_bits) {
            let cur_window_end = min(cur_window_start + window_size_in_bits, scalar_size_in_bits);
            let mut window_value = bits_to_usize(&scalars[i][cur_window_start..cur_window_end]) + (carry as usize);
            carry = (window_value >> (window_size_in_bits - 1)) > 0;
            window_value &= num_buckets - 1;
            window_vec.push(window_value);
        }
        if carry {
            window_vec.push(1);
            extra_window = true;
        }
        window_vecs.push(window_vec);
    }

    if extra_window {
        num_windows += 1;
    }

    let num_effective_buckets = num_buckets / 2; // only `buckets[1..2^(window_size_in_bits-1)]` will be used.
    let mut ret = S::zero();
    for window_id in (0..num_windows).rev() {
        let mut buckets = vec![S::zero(); num_effective_buckets];

        for i in 0..num_elements {
            let window_value = *window_vecs[i].get(window_id).unwrap_or(&0);
            let rotated_window_value = if window_value >= num_buckets >> 1 {
                (window_value as isize) - (num_buckets as isize)
            } else {
                window_value as isize
            };
            if rotated_window_value < 0 {
                buckets[(-rotated_window_value-1) as usize].add_assign(&element_negs[i]);
            } else if window_value > 0 {
                buckets[(rotated_window_value-1) as usize].add_assign(&elements[i]);
            }
        }

        let mut bucket_sum = S::zero();
        let mut weighted_bucket_sum = S::zero();
        for i in (0..num_effective_buckets).rev() {
            bucket_sum.add_assign(&buckets[i]);
            weighted_bucket_sum.add_assign(&bucket_sum);
        }
        for _ in 0..window_size_in_bits {
            ret.double_assign();
        }

        ret.add_assign(&weighted_bucket_sum);
    }

    ret
}

#[derive(Eq, PartialEq, Debug, Clone)]
struct I64Wrapper {
    val: i64
}

impl From<i64> for I64Wrapper {
    fn from(value: i64) -> Self {
        Self {
            val: value
        }
    }
}

impl PippengerFriendlyStructure for I64Wrapper {
    fn add(&self, other: &Self) -> Self {
        Self {
            val: self.val + other.val
        }
    }

    fn add_assign(&mut self, other: &Self) {
        self.val += other.val
    }

    fn double(&self) -> Self {
        Self {
            val: self.val * 2
        }
    }

    fn double_assign(&mut self) {
        self.val *= 2
    }

    fn neg(&self) -> Self {
        Self {
            val: -self.val
        }
    }

    fn zero() -> Self {
        Self {
            val: 0
        }
    }
}

#[test]
fn test_probably_pippenger() {
    let elements = vec![I64Wrapper::from(2), I64Wrapper::from(5), I64Wrapper::from(7)];
    let scalars = vec![usize_to_bits(10, 8), usize_to_bits(20, 8), usize_to_bits(30, 8)];
    assert_eq!(I64Wrapper::from(330), probably_pippenger_auto_window_size(elements.as_slice(), scalars.as_slice()));
}

#[test]
fn test_probably_pippenger_signed_digits() {
    let elements = vec![I64Wrapper::from(2), I64Wrapper::from(5), I64Wrapper::from(7)];
    let scalars = vec![usize_to_bits(10, 6), usize_to_bits(20, 6), usize_to_bits(30, 6)];
    assert_eq!(I64Wrapper::from(330), probably_pippenger_signed_digits(elements.as_slice(), scalars.as_slice(), 3));
}
