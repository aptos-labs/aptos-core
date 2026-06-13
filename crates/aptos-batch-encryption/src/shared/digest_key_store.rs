// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//! Streaming, bounded-residency reader for the on-disk `DigestKey` trusted setup.
//!
//! The trusted setup file is large (hundreds of MB to several GB). Loading it all into RAM was
//! the previous design (`Lazy<Option<Arc<DigestKey>>>`); this module replaces that with a
//! windowed loader:
//!
//! - **Pinned prefix**: rounds `[0, pinned_prefix_rounds)` are loaded at startup and never
//!   evicted. This makes epoch wrap (consumer going back to round 0) instant — no cold-start I/O.
//! - **Sliding window**: rounds `>= pinned_prefix_rounds` are tracked in a window
//!   `[cursor - sliding_lookback_rounds, cursor + sliding_lookahead_rounds]`. As the consumer
//!   advances `cursor`, the background loader prefetches ahead and evicts behind.
//!
//! Every round read is decoded via [`decode_round_from_slice`] from a batched read buffer
//! (`read_batch_rounds × round_size` bytes per I/O), amortizing syscall cost across rounds.
//!
//! The store implements [`DigestKeyView`] so it is a drop-in replacement for `&DigestKey` in
//! per-round consumers (`IdSet::compute_*_eval_proofs_with_setup`, FPTX scheme `digest`/`setup`).

use crate::shared::{
    algebra::fk_algorithm::FKDomainParams,
    digest::{DigestKeyHeader, DigestKeyView, RoundData},
    digest_key_file::{decode_round_from_slice, Header, HeaderV1},
};
use anyhow::{anyhow, bail, Result};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

/// Tuning knobs for [`DigestKeyStore`]. All fields are in rounds (not bytes) so the operator
/// chooses memory consumption directly.
#[derive(Debug, Clone)]
pub struct DigestKeyStoreConfig {
    /// Rounds `[0, pinned_prefix_rounds)` are loaded at startup and pinned in memory. Sized
    /// to cover the steady-state hot prefix; epoch wrap to round 0 stays instant because
    /// these slots never get evicted.
    pub pinned_prefix_rounds: usize,
    /// When the consumer is past the pinned prefix, the loader keeps this many rounds behind
    /// `cursor` resident in the sliding tier so a small backward step doesn't refault.
    pub sliding_lookback_rounds: usize,
    /// Loader pulls this many rounds ahead of `cursor` in the sliding tier. Tune so the
    /// loader stays ahead of the consumer's round-consumption rate.
    pub sliding_lookahead_rounds: usize,
    /// Per-syscall batch size. The loader reads `read_batch_rounds * round_size_bytes` bytes
    /// at a time into a scratch buffer, then decodes each round and publishes it.
    pub read_batch_rounds: usize,
}

impl Default for DigestKeyStoreConfig {
    fn default() -> Self {
        Self {
            pinned_prefix_rounds: 10_000,
            sliding_lookback_rounds: 100,
            sliding_lookahead_rounds: 100,
            read_batch_rounds: 64,
        }
    }
}

/// Streaming, bounded-residency view over a `DigestKey` blob file.
pub struct DigestKeyStore {
    inner: Arc<Inner>,
    _loader_join: Mutex<Option<thread::JoinHandle<()>>>,
}

struct Inner {
    header: Arc<DigestKeyHeader>,
    /// On-disk header. Used by the loader to compute round size and seek positions.
    file_header_v1: HeaderV1,
    /// Resolved round count on disk = `min(file_header.num_rounds, ...)`. Equal to
    /// `file_header_v1.num_rounds` today; carried separately so it's clear that `header.num_rounds`
    /// (the view trait surface) is the *resident* round count, not the on-disk count.
    num_rounds_on_disk: usize,
    /// Effective pinned prefix length = `min(cfg.pinned_prefix_rounds, num_rounds_on_disk)`.
    pinned_prefix_len: usize,
    /// Round-indexed slots for the pinned prefix. `None` until the loader publishes a slot;
    /// a fast consumer can block on `loader_wake` waiting for one to be filled.
    pinned: Vec<RwLock<Option<Arc<RoundData>>>>,
    /// Sliding tier — only used for rounds `>= pinned_prefix_len`.
    sliding: RwLock<BTreeMap<usize, Arc<RoundData>>>,
    /// Highest round the consumer has touched. The loader uses this to compute the sliding
    /// window target.
    cursor: AtomicUsize,
    /// `pinned_prefix_len` once the pinned prefix is fully loaded. Used as a fast path so
    /// pinned readers don't grab the condvar.
    pinned_loaded_through: AtomicUsize,
    cfg: DigestKeyStoreConfig,
    path: PathBuf,
    shutdown: AtomicBool,
    /// Notified on every publish (pinned or sliding) and on every `advance` / `shutdown`.
    loader_wake: Condvar,
    /// Guards the condvar wait condition only. Coarse but correct: anyone waiting on `round(r)`
    /// holds this for the duration of the wait.
    wait_lock: Mutex<()>,
}

impl DigestKeyStore {
    pub fn open(path: &Path, cfg: DigestKeyStoreConfig) -> Result<Arc<Self>> {
        let mut file = File::open(path)
            .map_err(|e| anyhow!("DigestKeyStore: failed to open {}: {}", path.display(), e))?;

        let mut header_buf = vec![0u8; Header::representation_size_bytes()];
        file.read_exact(&mut header_buf)
            .map_err(|e| anyhow!("DigestKeyStore: failed to read header: {}", e))?;
        let header_v1 = match bcs::from_bytes::<Header>(&header_buf)? {
            Header::V1(h) => h,
        };

        let expected_size_bytes = Header::representation_size_bytes()
            + header_v1.round_size_bytes() * header_v1.num_rounds;
        let actual = file.metadata()?.len() as usize;
        if actual != expected_size_bytes {
            bail!(
                "DigestKeyStore: file size mismatch — expected {} bytes ({} rounds), got {} bytes",
                expected_size_bytes,
                header_v1.num_rounds,
                actual
            );
        }

        let num_rounds_on_disk = header_v1.num_rounds;
        let pinned_prefix_len = cfg.pinned_prefix_rounds.min(num_rounds_on_disk);

        let view_header = Arc::new(DigestKeyHeader {
            tau_g2: header_v1.tau_g2,
            batch_size: header_v1.batch_size,
            num_rounds: num_rounds_on_disk,
            fk_params: FKDomainParams {
                toeplitz_domain: header_v1.toeplitz_domain.clone(),
                fft_domain: header_v1.fft_domain,
            },
        });

        let mut pinned = Vec::with_capacity(pinned_prefix_len);
        for _ in 0..pinned_prefix_len {
            pinned.push(RwLock::new(None));
        }

        let inner = Arc::new(Inner {
            header: view_header,
            file_header_v1: header_v1,
            num_rounds_on_disk,
            pinned_prefix_len,
            pinned,
            sliding: RwLock::new(BTreeMap::new()),
            cursor: AtomicUsize::new(0),
            pinned_loaded_through: AtomicUsize::new(0),
            cfg,
            path: path.to_path_buf(),
            shutdown: AtomicBool::new(false),
            loader_wake: Condvar::new(),
            wait_lock: Mutex::new(()),
        });

        let inner_for_loader = Arc::clone(&inner);
        let join = thread::Builder::new()
            .name("digest-key-loader".into())
            .spawn(move || loader_main(inner_for_loader))?;

        Ok(Arc::new(Self {
            inner,
            _loader_join: Mutex::new(Some(join)),
        }))
    }

    /// Hint to the loader that the consumer has advanced to round `r`. Auto-called by
    /// [`DigestKeyStore::round`] too, so explicit calls are only needed when the consumer
    /// wants to drive prefetch without actually fetching.
    pub fn advance(&self, r: usize) {
        let prev = self.inner.cursor.load(Ordering::Relaxed);
        if r > prev {
            self.inner.cursor.store(r, Ordering::Relaxed);
            self.inner.loader_wake.notify_all();
        }
    }

    /// Number of rounds available on disk.
    pub fn num_rounds_on_disk(&self) -> usize {
        self.inner.num_rounds_on_disk
    }

    /// Block until the pinned prefix is fully loaded. Used at startup so consumers don't race
    /// the loader for the first few rounds.
    pub fn wait_pinned_ready(&self) {
        let target = self.inner.pinned_prefix_len;
        if self.inner.pinned_loaded_through.load(Ordering::Acquire) >= target {
            return;
        }
        let mut guard = self.inner.wait_lock.lock().unwrap();
        while self.inner.pinned_loaded_through.load(Ordering::Acquire) < target {
            guard = self.inner.loader_wake.wait(guard).unwrap();
        }
    }
}

impl Drop for DigestKeyStore {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::Release);
        self.inner.loader_wake.notify_all();
        if let Some(handle) = self._loader_join.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

impl DigestKeyView for DigestKeyStore {
    fn header(&self) -> &DigestKeyHeader {
        &self.inner.header
    }

    fn round(&self, r: usize) -> Arc<RoundData> {
        // Auto-advance cursor. Consumers don't have to remember to call `advance`.
        let prev = self.inner.cursor.load(Ordering::Relaxed);
        if r > prev {
            self.inner.cursor.store(r, Ordering::Relaxed);
            self.inner.loader_wake.notify_all();
        }

        if r < self.inner.pinned_prefix_len {
            if let Some(arc) = self
                .inner
                .pinned
                .get(r)
                .and_then(|slot| slot.read().unwrap().clone())
            {
                return arc;
            }
            // Slow path: pinned slot not yet loaded.
            let mut guard = self.inner.wait_lock.lock().unwrap();
            loop {
                if let Some(arc) = self.inner.pinned[r].read().unwrap().clone() {
                    return arc;
                }
                guard = self.inner.loader_wake.wait(guard).unwrap();
            }
        }

        // Sliding tier.
        if let Some(arc) = self.inner.sliding.read().unwrap().get(&r).cloned() {
            return arc;
        }
        let mut guard = self.inner.wait_lock.lock().unwrap();
        loop {
            if let Some(arc) = self.inner.sliding.read().unwrap().get(&r).cloned() {
                return arc;
            }
            guard = self.inner.loader_wake.wait(guard).unwrap();
        }
    }
}

fn loader_main(inner: Arc<Inner>) {
    let result = loader_main_inner(&inner);
    if let Err(e) = result {
        // We have no good way to surface this to the consumer (they're parked on
        // `loader_wake`). Wake them so they exit their wait loops and observe missing rounds.
        tracing::error!("DigestKeyStore loader thread failed: {}", e);
        inner.shutdown.store(true, Ordering::Release);
        inner.loader_wake.notify_all();
    }
}

fn loader_main_inner(inner: &Inner) -> Result<()> {
    let header_v1 = inner.file_header_v1.clone();
    let round_size = header_v1.round_size_bytes();
    let header_offset = Header::representation_size_bytes();
    let batch_rounds = inner.cfg.read_batch_rounds.max(1);

    let mut file = File::open(&inner.path)?;
    file.seek(SeekFrom::Start(header_offset as u64))?;

    let mut scratch = vec![0u8; batch_rounds * round_size];
    let mut file_cursor_round: usize = 0;

    // Phase 1: pinned prefix, strictly forward.
    while file_cursor_round < inner.pinned_prefix_len {
        if inner.shutdown.load(Ordering::Acquire) {
            return Ok(());
        }
        let remaining_in_pinned = inner.pinned_prefix_len - file_cursor_round;
        let this_batch = batch_rounds.min(remaining_in_pinned);
        let bytes = this_batch * round_size;
        file.read_exact(&mut scratch[..bytes])?;

        for i in 0..this_batch {
            let r = file_cursor_round + i;
            let round_bytes = &scratch[i * round_size..(i + 1) * round_size];
            let round_file = decode_round_from_slice(round_bytes, &header_v1)?;
            let arc = Arc::new(RoundData {
                tau_powers_g1: round_file.tau_powers_g1,
                prepared_toeplitz_input: round_file.prepared_toeplitz_input,
            });
            *inner.pinned[r].write().unwrap() = Some(arc);
            inner.pinned_loaded_through.store(r + 1, Ordering::Release);
            inner.loader_wake.notify_all();
        }
        file_cursor_round += this_batch;
    }

    // Phase 2: sliding tier maintenance.
    // `file_cursor_round` is the next round the file's read pointer will deliver. When the
    // consumer's window asks for a round we haven't seen, we may need to seek backward (rare)
    // or jump forward.
    let lookback = inner.cfg.sliding_lookback_rounds;
    let lookahead = inner.cfg.sliding_lookahead_rounds;
    let num_rounds = inner.num_rounds_on_disk;
    let pinned_len = inner.pinned_prefix_len;

    loop {
        if inner.shutdown.load(Ordering::Acquire) {
            return Ok(());
        }
        let cursor = inner.cursor.load(Ordering::Acquire);
        // Sliding tier only covers rounds >= pinned_len.
        let window_lo = cursor.saturating_sub(lookback).max(pinned_len);
        let window_hi_excl = (cursor + lookahead + 1).min(num_rounds);

        // Evict everything outside the window.
        {
            let mut sliding = inner.sliding.write().unwrap();
            // BTreeMap retain isn't stable; collect keys to drop.
            let drop_keys: Vec<usize> = sliding
                .range(..window_lo)
                .chain(sliding.range(window_hi_excl..))
                .map(|(k, _)| *k)
                .collect();
            for k in drop_keys {
                sliding.remove(&k);
            }
        }

        if window_lo >= window_hi_excl {
            // No sliding work — consumer is still in the pinned region, or past EOF.
            let guard = inner.wait_lock.lock().unwrap();
            // Park, but wake periodically as a safety net in case a notify was missed.
            let _ = inner
                .loader_wake
                .wait_timeout(guard, Duration::from_millis(50))
                .unwrap();
            continue;
        }

        // Find the next round in [window_lo, window_hi_excl) that isn't resident.
        let next_missing = {
            let sliding = inner.sliding.read().unwrap();
            (window_lo..window_hi_excl).find(|r| !sliding.contains_key(r))
        };

        let Some(r) = next_missing else {
            // Window is fully resident; park.
            let guard = inner.wait_lock.lock().unwrap();
            let _ = inner
                .loader_wake
                .wait_timeout(guard, Duration::from_millis(50))
                .unwrap();
            continue;
        };

        // Seek the file cursor to `r` if needed.
        if file_cursor_round != r {
            let offset = header_offset as u64 + (r as u64) * (round_size as u64);
            file.seek(SeekFrom::Start(offset))?;
            file_cursor_round = r;
        }

        // Decide how many contiguous missing rounds we can batch-read starting at `r`.
        let max_batch_end = (r + batch_rounds).min(window_hi_excl);
        let batch_end = {
            let sliding = inner.sliding.read().unwrap();
            (r..max_batch_end)
                .take_while(|i| !sliding.contains_key(i))
                .last()
                .map(|last| last + 1)
                .unwrap_or(r + 1)
        };
        let this_batch = batch_end - r;
        let bytes = this_batch * round_size;
        file.read_exact(&mut scratch[..bytes])?;

        for i in 0..this_batch {
            let round = r + i;
            let round_bytes = &scratch[i * round_size..(i + 1) * round_size];
            let round_file = decode_round_from_slice(round_bytes, &header_v1)?;
            let arc = Arc::new(RoundData {
                tau_powers_g1: round_file.tau_powers_g1,
                prepared_toeplitz_input: round_file.prepared_toeplitz_input,
            });
            inner.sliding.write().unwrap().insert(round, arc);
            inner.loader_wake.notify_all();
        }
        file_cursor_round += this_batch;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{digest::DigestKey, digest_key_file::write_digest_key};
    use ark_std::rand::thread_rng;
    use tempfile::NamedTempFile;

    fn make_setup(batch_size: usize, num_rounds: usize) -> (NamedTempFile, DigestKey) {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, batch_size, num_rounds).unwrap();
        let file = NamedTempFile::new().unwrap();
        write_digest_key(file.path(), dk.clone()).unwrap();
        (file, dk)
    }

    #[test]
    fn sequential_walk_matches_eager_load() {
        let (file, dk) = make_setup(8, 12);
        let cfg = DigestKeyStoreConfig {
            pinned_prefix_rounds: 4,
            sliding_lookback_rounds: 2,
            sliding_lookahead_rounds: 2,
            read_batch_rounds: 3,
        };
        let store = DigestKeyStore::open(file.path(), cfg).unwrap();
        store.wait_pinned_ready();

        assert_eq!(store.num_rounds(), dk.num_rounds());
        assert_eq!(store.tau_g2(), dk.tau_g2());
        for r in 0..dk.num_rounds() {
            let from_store = store.round(r);
            let from_dk = dk.round_data(r);
            assert_eq!(*from_store, *from_dk, "round {} mismatch", r);
        }
    }

    #[test]
    fn epoch_wrap_round_zero_stays_pinned() {
        let (file, dk) = make_setup(8, 8);
        let cfg = DigestKeyStoreConfig {
            pinned_prefix_rounds: 4,
            sliding_lookback_rounds: 1,
            sliding_lookahead_rounds: 1,
            read_batch_rounds: 2,
        };
        let store = DigestKeyStore::open(file.path(), cfg).unwrap();
        store.wait_pinned_ready();
        // Walk into the sliding tier.
        for r in 0..8 {
            let _ = store.round(r);
        }
        // Wrap. Round 0 must still be pinned and instantly available.
        let from_store_zero = store.round(0);
        assert_eq!(*from_store_zero, *dk.round_data(0));
    }

    #[test]
    fn full_file_pinned_no_sliding_work() {
        let (file, dk) = make_setup(8, 5);
        let cfg = DigestKeyStoreConfig {
            pinned_prefix_rounds: 100,
            sliding_lookback_rounds: 5,
            sliding_lookahead_rounds: 5,
            read_batch_rounds: 2,
        };
        let store = DigestKeyStore::open(file.path(), cfg).unwrap();
        store.wait_pinned_ready();
        for r in 0..dk.num_rounds() {
            assert_eq!(*store.round(r), *dk.round_data(r));
        }
    }
}
