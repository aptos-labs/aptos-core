// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::ensure;
use velor_logger::warn;
use core::fmt;
use serde::Serialize;
use std::cmp::{max, Ordering};

/// This struct always ensures the following invariants:
/// * count <= bytes
/// * (count > 0 && bytes > 0) || (count == 0 && bytes == 0)
#[derive(Debug, Clone, Copy, Serialize, Default)]
pub struct PayloadTxnsSize {
    count: u64,
    bytes: u64,
}

impl PayloadTxnsSize {
    pub fn new(count: u64, bytes: u64) -> Self {
        match Self::try_new(count, bytes) {
            Ok(txns_size) => txns_size,
            Err(err) => {
                warn!(
                    "Invalid input for PayloadTxnsSize. Normalizing. Count: {}, Bytes: {}, Err: {}",
                    count, bytes, err
                );
                Self::new_normalized(count, bytes)
            },
        }
    }

    fn new_normalized(count: u64, bytes: u64) -> Self {
        let mut count = count;
        let mut bytes = bytes;
        if count > bytes {
            bytes = count;
        }
        if count == 0 || bytes == 0 {
            count = 0;
            bytes = 0;
        }
        Self { count, bytes }
    }

    fn try_new(count: u64, bytes: u64) -> anyhow::Result<Self> {
        ensure!(count <= bytes);
        ensure!((count > 0 && bytes > 0) || (count == 0 && bytes == 0));

        Ok(Self { count, bytes })
    }

    pub fn zero() -> Self {
        Self { count: 0, bytes: 0 }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn size_in_bytes(&self) -> u64 {
        self.bytes
    }

    pub fn compute_pct(self, pct: u8) -> Self {
        Self::new_normalized(self.count * pct as u64 / 100, self.bytes * pct as u64 / 100)
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self::new_normalized(
            self.count.saturating_sub(rhs.count),
            self.bytes.saturating_sub(rhs.bytes),
        )
    }

    pub fn set_count(&mut self, new_count: u64) {
        if let Err(e) = self.try_set_count(new_count) {
            warn!(
                "Invalid set count. Resetting bytes. new_count: {}, Self: {}, Error: {}",
                new_count, self, e
            );
            *self = Self::new_normalized(new_count, new_count);
        }
    }

    pub fn try_set_count(&mut self, new_count: u64) -> anyhow::Result<()> {
        *self = Self::try_new(new_count, self.bytes)?;
        Ok(())
    }

    /// Computes a new [PayloadTxnsSize] whose size in bytes is the passed-in value and the
    /// count is calculated proportional to bytes. If the existing PayloadTxnsSize is zero
    /// then the new size replaces both the count and size in bytes.
    pub fn compute_with_bytes(&self, new_size_in_bytes: u64) -> PayloadTxnsSize {
        let new_count = if self.bytes > 0 {
            let factor = new_size_in_bytes as f64 / self.bytes as f64;
            max((self.count as f64 * factor) as u64, 1u64)
        } else {
            // If bytes is zero, then count is zero. In this case, set the new
            // count to be the same as bytes.
            new_size_in_bytes
        };
        PayloadTxnsSize::new_normalized(new_count, new_size_in_bytes)
    }

    pub fn minimum(self, other: Self) -> Self {
        let count = self.count.min(other.count);
        let bytes = self.bytes.min(other.bytes);
        PayloadTxnsSize::new_normalized(count, bytes)
    }

    pub fn maximum(self, other: Self) -> Self {
        let count = self.count.max(other.count);
        let bytes = self.bytes.max(other.bytes);
        PayloadTxnsSize::new_normalized(count, bytes)
    }
}

impl std::ops::Add for PayloadTxnsSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new_normalized(self.count + rhs.count, self.bytes + rhs.bytes)
    }
}

impl std::ops::AddAssign for PayloadTxnsSize {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self::new_normalized(self.count + rhs.count, self.bytes + rhs.bytes);
    }
}

impl std::ops::Sub for PayloadTxnsSize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new_normalized(self.count - rhs.count, self.bytes - rhs.bytes)
    }
}

impl std::ops::SubAssign for PayloadTxnsSize {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self::new_normalized(self.count - rhs.count, self.bytes - rhs.bytes);
    }
}

impl PartialEq for PayloadTxnsSize {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count && self.bytes == other.bytes
    }
}

impl PartialOrd for PayloadTxnsSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.count == other.count && self.bytes == other.bytes {
            return Some(Ordering::Equal);
        }

        if self.count > other.count || self.bytes > other.bytes {
            return Some(Ordering::Greater);
        }

        if self.count < other.count && self.bytes < other.bytes {
            return Some(Ordering::Less);
        }

        None
    }
}

impl Eq for PayloadTxnsSize {}

impl fmt::Display for PayloadTxnsSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PayloadTxnsSize[count: {}, bytes: {}]",
            self.count, self.bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::PayloadTxnsSize;

    #[test]
    fn test_payload_txns_size_operations() {
        let txns_size = PayloadTxnsSize::new(100, 100);
        assert_eq!(txns_size.compute_pct(90), PayloadTxnsSize::new(90, 90));
        assert_eq!(txns_size.compute_pct(50), PayloadTxnsSize::new(50, 50));

        let mut txns_size2 = txns_size;
        txns_size2.set_count(50);
        assert_eq!(txns_size2, PayloadTxnsSize::new(50, 100));
        txns_size2.set_count(200);
        assert_eq!(txns_size2, PayloadTxnsSize::new(200, 200));

        let txns_size3 = txns_size;
        let txns_size4 = txns_size;
        assert_eq!(txns_size3 + txns_size4, PayloadTxnsSize::new(200, 200));
        assert_eq!(txns_size3 - txns_size4, PayloadTxnsSize::zero());

        let mut txns_size5 = txns_size;
        txns_size5 += txns_size3;
        assert_eq!(txns_size5, PayloadTxnsSize::new(200, 200));
        txns_size5 -= txns_size3;
        assert_eq!(txns_size5, PayloadTxnsSize::new(100, 100));

        assert_eq!(
            txns_size.compute_with_bytes(200),
            PayloadTxnsSize::new(200, 200)
        );
        assert_eq!(
            txns_size.compute_with_bytes(50),
            PayloadTxnsSize::new(50, 50)
        );

        assert_eq!(
            txns_size.saturating_sub(txns_size2),
            PayloadTxnsSize::zero()
        );
        assert_eq!(
            txns_size2.saturating_sub(txns_size),
            PayloadTxnsSize::new(100, 100)
        );

        let txns_size5 = PayloadTxnsSize::zero();
        assert_eq!(
            txns_size5.compute_with_bytes(100),
            PayloadTxnsSize::new(100, 100)
        );

        let txns_size6 = PayloadTxnsSize::new(10, 30);
        let txns_size7 = PayloadTxnsSize::new(20, 20);
        assert_eq!(txns_size6.minimum(txns_size7), PayloadTxnsSize::new(10, 20));
        assert_eq!(txns_size6.maximum(txns_size7), PayloadTxnsSize::new(20, 30));

        assert_eq!(
            txns_size6.saturating_sub(txns_size7),
            PayloadTxnsSize::zero()
        );

        assert_eq!(
            PayloadTxnsSize::try_new(100, 0).unwrap_err().to_string(),
            "Condition failed: `count <= bytes` (100 vs 0)"
        );
        assert_eq!(
            PayloadTxnsSize::try_new(100, 10).unwrap_err().to_string(),
            "Condition failed: `count <= bytes` (100 vs 10)"
        );

        let mut txns_size8 = txns_size;
        assert_eq!(
            txns_size8.try_set_count(200).unwrap_err().to_string(),
            "Condition failed: `count <= bytes` (200 vs 100)"
        );
        txns_size8.set_count(200);
        assert_eq!(txns_size8, PayloadTxnsSize::new(200, 200));

        let txns_size9 = PayloadTxnsSize::new(3, 3000);
        let txns_size10 = PayloadTxnsSize::new(2, 100);
        let txns_size11 = PayloadTxnsSize::new(2, 200);
        assert!(txns_size10 + txns_size11 > txns_size9);
    }
}
