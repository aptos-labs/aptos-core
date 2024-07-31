#[derive(Debug, Clone, Copy)]
pub struct PayloadTxnsSize {
    pub count: u64,
    pub bytes: u64,
}

impl PayloadTxnsSize {
    pub fn new(count: u64, bytes: u64) -> Self {
        assert!(count <= bytes);
        Self { count, bytes }
    }

    pub fn zero() -> Self {
        Self { count: 0, bytes: 0 }
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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for PayloadTxnsSize {}

impl Ord for PayloadTxnsSize {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        assert!(self.count <= self.bytes || other.count <= other.bytes);

        if self.count == other.count && self.bytes == other.bytes {
            return std::cmp::Ordering::Equal;
        }

        if self.count > other.count && self.bytes > other.bytes {
            return std::cmp::Ordering::Greater;
        }

        std::cmp::Ordering::Less
    }
}
