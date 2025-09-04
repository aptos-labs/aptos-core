// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

/// For all $n > 0$, returns $\floor{\log_2{n}}$, contained within a `Some`.
/// For $n = 0$, returns `None`.
pub fn log2_floor(n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }

    // NOTE: n > 0, so n.leading_zeros() cannot equal usize::BITS. Therefore, we will never cast -1 to a usize.
    Some(((usize::BITS - n.leading_zeros()) - 1) as usize)
}

#[test]
fn test_log2_floor() {
    assert_eq!(log2_floor(usize::MIN), None);
    assert_eq!(log2_floor(0), None);
    assert_eq!(log2_floor(1), Some(0));
    assert_eq!(log2_floor(2), Some(1));
    assert_eq!(log2_floor(3), Some(1));
    assert_eq!(log2_floor(4), Some(2));
    assert_eq!(log2_floor(5), Some(2));
    assert_eq!(log2_floor(6), Some(2));
    assert_eq!(log2_floor(7), Some(2));
    assert_eq!(log2_floor(8), Some(3));
    assert_eq!(log2_floor(9), Some(3));
    assert_eq!(log2_floor(10), Some(3));
    assert_eq!(log2_floor(11), Some(3));
    assert_eq!(log2_floor(12), Some(3));
    assert_eq!(log2_floor(13), Some(3));
    assert_eq!(log2_floor(14), Some(3));
    assert_eq!(log2_floor(15), Some(3));
    assert_eq!(log2_floor(16), Some(4));
    assert_eq!(log2_floor(usize::MAX), Some((usize::BITS - 1) as usize));
}

/// For all $n > 0$, returns $\ceil{\log_2{n}}$, contained within a `Some`.
/// For $n = 0$, returns `None`.
pub fn log2_ceil(n: usize) -> Option<usize> {
    match n {
        0 => None,
        1 => Some(0),
        _ => Some(log2_floor(n - 1).unwrap() + 1),
    }
}

#[test]
fn test_log2_ceil() {
    assert_eq!(log2_ceil(usize::MIN), None);
    assert_eq!(log2_ceil(0), None);
    assert_eq!(log2_ceil(1), Some(0));
    assert_eq!(log2_ceil(2), Some(1));
    assert_eq!(log2_ceil(3), Some(2));
    assert_eq!(log2_ceil(4), Some(2));
    assert_eq!(log2_ceil(5), Some(3));
    assert_eq!(log2_ceil(6), Some(3));
    assert_eq!(log2_ceil(7), Some(3));
    assert_eq!(log2_ceil(8), Some(3));
    assert_eq!(log2_ceil(9), Some(4));
    assert_eq!(log2_ceil(10), Some(4));
    assert_eq!(log2_ceil(11), Some(4));
    assert_eq!(log2_ceil(12), Some(4));
    assert_eq!(log2_ceil(13), Some(4));
    assert_eq!(log2_ceil(14), Some(4));
    assert_eq!(log2_ceil(15), Some(4));
    assert_eq!(log2_ceil(16), Some(4));
    assert_eq!(log2_ceil(usize::MAX), Some(usize::BITS as usize));
}
