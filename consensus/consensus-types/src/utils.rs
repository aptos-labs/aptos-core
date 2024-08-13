use anyhow::ensure;
use core::fmt;
use serde::Serialize;
use std::cmp::{max, Ordering};

#[derive(Debug, Clone, Copy, Serialize, Default)]
pub struct PayloadTxnsSize {
    count: u64,
    bytes: u64,
}

impl PayloadTxnsSize {
    pub fn new(count: u64, bytes: u64) -> Self {
        assert!(count <= bytes, "count: {} > bytes: {}", count, bytes);
        assert!((count > 0 && bytes > 0) || (count == 0 && bytes == 0));
        Self { count, bytes }
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
        Self {
            count: self.count * pct as u64 / 100,
            bytes: self.bytes * pct as u64 / 100,
        }
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self {
            count: self.count.saturating_sub(rhs.count),
            bytes: self.bytes.saturating_sub(rhs.bytes),
        }
    }

    pub fn set_count(&mut self, new_count: u64) {
        if let Err(e) = self.try_set_count(new_count) {
            error!(
                "Invalid set count. Resetting bytes. new_count: {}, Error: {}",
                new_count, e
            );
            self.count = new_count;
            self.bytes = new_count;
        }
    }

    pub fn try_set_count(&mut self, new_count: u64) -> anyhow::Result<()> {
        ensure!(new_count <= self.bytes);
        self.count = new_count;
        if new_count == 0 {
            self.bytes = 0;
        }
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
        PayloadTxnsSize::new(new_count, new_size_in_bytes)
    }

    pub fn minimum(self, other: Self) -> Self {
        match self.partial_cmp(&other) {
            Some(order) => match order {
                Ordering::Less => self,
                Ordering::Equal => self,
                Ordering::Greater => other,
            },
            None => PayloadTxnsSize::new(self.count.min(other.count), self.bytes.min(other.bytes)),
        }
    }

    pub fn maximum(self, other: Self) -> Self {
        match self.partial_cmp(&other) {
            Some(order) => match order {
                Ordering::Less => other,
                Ordering::Equal => self,
                Ordering::Greater => self,
            },
            None => PayloadTxnsSize::new(self.count.max(other.count), self.bytes.max(other.bytes)),
        }
    }
}

impl std::ops::Add for PayloadTxnsSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            count: self.count + rhs.count,
            bytes: self.bytes + rhs.bytes,
        }
    }
}

impl std::ops::AddAssign for PayloadTxnsSize {
    fn add_assign(&mut self, rhs: Self) {
        self.count += rhs.count;
        self.bytes += rhs.bytes;
    }
}

impl std::ops::Sub for PayloadTxnsSize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            count: self.count - rhs.count,
            bytes: self.bytes - rhs.bytes,
        }
    }
}

impl std::ops::SubAssign for PayloadTxnsSize {
    fn sub_assign(&mut self, rhs: Self) {
        self.count -= rhs.count;
        self.bytes -= rhs.bytes;
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

        if self.count > other.count && self.bytes > other.bytes {
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
    }
}
