// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use cfg_if::cfg_if;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::num::NonZeroUsize;

/// Cache for already verified modules. Since loader V1 uses such a cache to not perform repeated
/// verifications, possibly even across blocks, for comparative performance we need to have it as
/// well. For now, we keep it as a separate cache to make sure there is no interference between V1
/// and V2 implementations.
///
/// The cache stores a verifier config fingerprint alongside each entry. This ensures that if the
/// verifier configuration changes (e.g., stricter struct definition limits are enabled), modules
/// verified under the old configuration are not treated as verified under the new one. This is
/// critical for concurrent replay where different threads may process version ranges with
/// different verifier configs simultaneously.
pub(crate) struct VerifiedModuleCache(Mutex<lru::LruCache<[u8; 32], u64>>);

impl VerifiedModuleCache {
    /// Maximum size of the cache. When modules are cached, they can skip re-verification.
    const VERIFIED_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(100_000).unwrap();

    /// Returns new empty verified module cache.
    pub(crate) fn empty() -> Self {
        Self(Mutex::new(lru::LruCache::new(Self::VERIFIED_CACHE_SIZE)))
    }

    /// Returns true if the module hash is contained in the cache and was verified with a matching
    /// verifier config. For tests, the cache is treated as empty at all times.
    pub(crate) fn contains(
        &self,
        module_hash: &[u8; 32],
        verifier_config_fingerprint: u64,
    ) -> bool {
        // Note: need to use get to update LRU queue.
        verifier_cache_enabled()
            && self
                .0
                .lock()
                .get(module_hash)
                .is_some_and(|&stored| stored == verifier_config_fingerprint)
    }

    /// Inserts the hash into the cache, marking the corresponding module as locally verified
    /// under the given verifier config. For tests, entries are not added to the cache.
    pub(crate) fn put(&self, module_hash: [u8; 32], verifier_config_fingerprint: u64) {
        if verifier_cache_enabled() {
            let mut cache = self.0.lock();
            cache.put(module_hash, verifier_config_fingerprint);
        }
    }

    /// Flushes the verified modules cache.
    pub(crate) fn flush(&self) {
        self.0.lock().clear();
    }

    /// Returns the number of verified modules in the cache.
    pub(crate) fn size(&self) -> usize {
        self.0.lock().len()
    }
}

lazy_static! {
    pub(crate) static ref VERIFIED_MODULES_CACHE: VerifiedModuleCache =
        VerifiedModuleCache::empty();
}

#[cfg_attr(feature = "force-inline", inline(always))]
fn verifier_cache_enabled() -> bool {
    cfg_if! {
        if #[cfg(feature = "disable_verifier_cache")] {
            false
        } else {
            // Cache is enabled in non-test environments only.
            true
        }
    }
}
