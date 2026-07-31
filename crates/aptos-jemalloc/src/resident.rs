// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Throttle-cached reader for process resident bytes, used by the API memory
//! admission gate. The pure `ResidentCache` takes its clock and raw reader as
//! parameters so it is unit-testable without jemalloc or a live clock.

#[cfg(any(unix, test))]
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// Minimum interval between underlying jemalloc reads. Requests within one window
/// approximately share a sampled value; at a boundary a few concurrent refreshes
/// may race, which is harmless for a best-effort gate.
#[cfg(any(unix, test))]
const REFRESH_INTERVAL_MILLIS: u64 = 250;

/// Throttled cache of process resident bytes.
#[cfg(any(unix, test))]
struct ResidentCache {
    /// Last sampled resident bytes; negative means "never sampled".
    bytes: AtomicI64,
    /// Monotonic-millis timestamp of the last refresh; `u64::MAX` means "never".
    last_refresh_millis: AtomicU64,
}

#[cfg(any(unix, test))]
impl ResidentCache {
    const fn new() -> Self {
        Self {
            bytes: AtomicI64::new(-1),
            last_refresh_millis: AtomicU64::new(u64::MAX),
        }
    }

    /// Returns cached resident bytes, refreshing via `read_raw` only when the cache
    /// is older than `REFRESH_INTERVAL_MILLIS`. `now_millis` is a monotonic clock
    /// reading. Returns `None` until a successful read has ever occurred.
    fn get(&self, now_millis: u64, read_raw: impl FnOnce() -> Option<i64>) -> Option<i64> {
        // `bytes` and `last_refresh_millis` are updated independently under Relaxed:
        // brief staleness and a few redundant reads per window are tolerated by design.
        let last = self.last_refresh_millis.load(Ordering::Relaxed);
        let stale = last == u64::MAX || now_millis.saturating_sub(last) >= REFRESH_INTERVAL_MILLIS;
        if stale && let Some(v) = read_raw() {
            self.bytes.store(v, Ordering::Relaxed);
            self.last_refresh_millis
                .store(now_millis, Ordering::Relaxed);
            return Some(v);
        }
        match self.bytes.load(Ordering::Relaxed) {
            b if b >= 0 => Some(b),
            _ => None,
        }
    }
}

#[cfg(unix)]
static CACHE: ResidentCache = ResidentCache::new();

/// Current process resident bytes (jemalloc `stats.resident`), throttle-cached.
/// `None` when unavailable (non-unix, jemalloc not the global allocator, or read
/// error). Callers treat `None` as "unknown" and fail open.
#[cfg(unix)]
pub fn current_process_resident_bytes() -> Option<i64> {
    use once_cell::sync::Lazy;
    use std::time::Instant;
    static BASE: Lazy<Instant> = Lazy::new(Instant::now);
    let now_millis = BASE.elapsed().as_millis() as u64;
    CACHE.get(now_millis, read_resident_raw)
}

#[cfg(unix)]
fn read_resident_raw() -> Option<i64> {
    use jemalloc_ctl::{epoch, stats};
    // Advance the epoch so the stats read returns a fresh value.
    epoch::advance().ok()?;
    stats::resident::read().ok().map(|v| v as i64)
}

#[cfg(not(unix))]
pub fn current_process_resident_bytes() -> Option<i64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn first_call_refreshes_then_caches_within_interval() {
        let cache = ResidentCache::new();
        let calls = Cell::new(0);
        let read = || {
            calls.set(calls.get() + 1);
            Some(1000)
        };
        assert_eq!(cache.get(0, read), Some(1000));
        assert_eq!(calls.get(), 1);
        // Within the interval: cached, no new raw read.
        assert_eq!(cache.get(100, read), Some(1000));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn refreshes_after_interval() {
        let cache = ResidentCache::new();
        let next = Cell::new(1000i64);
        let read = || Some(next.get());
        assert_eq!(cache.get(0, read), Some(1000));
        next.set(2000);
        assert_eq!(cache.get(REFRESH_INTERVAL_MILLIS - 1, read), Some(1000));
        assert_eq!(cache.get(REFRESH_INTERVAL_MILLIS, read), Some(2000));
    }

    #[test]
    fn none_until_first_successful_read() {
        let cache = ResidentCache::new();
        assert_eq!(cache.get(0, || None), None);
        assert_eq!(cache.get(REFRESH_INTERVAL_MILLIS, || Some(500)), Some(500));
        // Not yet stale again: cached value returned without calling read_raw.
        assert_eq!(cache.get(REFRESH_INTERVAL_MILLIS + 1, || None), Some(500));
    }
}
