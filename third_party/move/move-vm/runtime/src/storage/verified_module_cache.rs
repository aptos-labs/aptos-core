// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use cfg_if::cfg_if;
use lazy_static::lazy_static;
use move_vm_metrics::VERIFIED_MODULE_CACHE_SIZE;
use parking_lot::Mutex;
use std::num::NonZeroUsize;

/// Cache for already verified modules. Since loader V1 uses such a cache to not perform repeated
/// verifications, possibly even across blocks, for comparative performance we need to have it as
/// well. For now, we keep it as a separate cache to make sure there is no interference between V1
/// and V2 implementations.
pub(crate) struct VerifiedModuleCache(Mutex<lru::LruCache<[u8; 32], ()>>);

impl VerifiedModuleCache {
    /// Maximum size of the cache. When modules are cached, they can skip re-verification.
    const VERIFIED_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(100_000).unwrap();

    /// Returns new empty verified module cache.
    pub(crate) fn empty() -> Self {
        Self(Mutex::new(lru::LruCache::new(Self::VERIFIED_CACHE_SIZE)))
    }

    /// Returns true if the module hash is contained in the cache. For tests, the cache is treated
    /// as empty at all times.
    pub(crate) fn contains(&self, module_hash: &[u8; 32]) -> bool {
        // Note: need to use get to update LRU queue.
        verifier_cache_enabled() && self.0.lock().get(module_hash).is_some()
    }

    /// Inserts the hash into the cache, marking the corresponding as locally verified. For tests,
    /// entries are not added to the cache.
    pub(crate) fn put(&self, module_hash: [u8; 32]) {
        if verifier_cache_enabled() {
            let mut cache = self.0.lock();
            cache.put(module_hash, ());
            VERIFIED_MODULE_CACHE_SIZE.set(cache.len() as i64);
        }
    }

    /// Flushes the verified modules cache.
    pub(crate) fn flush(&self) {
        if verifier_cache_enabled() {
            self.0.lock().clear();
        }
    }
}

lazy_static! {
    pub(crate) static ref VERIFIED_MODULES_CACHE: VerifiedModuleCache =
        VerifiedModuleCache::empty();
}

#[inline(always)]
fn verifier_cache_enabled() -> bool {
    cfg_if! {
        if #[cfg(any(test, feature = "testing"))] {
            false
        } else {
            // Cache is enabled in non-test environments only.
            true
        }
    }
}
