// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use cfg_if::cfg_if;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::{hash::Hash, num::NonZeroUsize};

/// Cache key combining the module hash with the verifier config hash, so that modules verified
/// under different configs get separate cache entries.
#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct VerifierCacheKey {
    module_hash: [u8; 32],
    verifier_config_hash: [u8; 32],
}

impl VerifierCacheKey {
    pub(crate) fn new(module_hash: [u8; 32], verifier_config_hash: [u8; 32]) -> Self {
        Self {
            module_hash,
            verifier_config_hash,
        }
    }
}

/// Cache for already verified modules. Since loader V1 uses such a cache to not perform repeated
/// verifications, possibly even across blocks, for comparative performance we need to have it as
/// well. For now, we keep it as a separate cache to make sure there is no interference between V1
/// and V2 implementations.
pub(crate) struct VerifiedModuleCache(Mutex<lru::LruCache<VerifierCacheKey, ()>>);

impl VerifiedModuleCache {
    /// Maximum size of the cache. When modules are cached, they can skip re-verification.
    const VERIFIED_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(100_000).unwrap();

    /// Returns new empty verified module cache.
    pub(crate) fn empty() -> Self {
        Self(Mutex::new(lru::LruCache::new(Self::VERIFIED_CACHE_SIZE)))
    }

    /// Returns true if the key is contained in the cache. For tests, the cache is treated as empty
    /// at all times.
    pub(crate) fn contains(&self, key: &VerifierCacheKey) -> bool {
        // Note: need to use get to update LRU queue.
        verifier_cache_enabled() && self.0.lock().get(key).is_some()
    }

    /// Inserts the key into the cache, marking the corresponding module as locally verified. For
    /// tests, entries are not added to the cache.
    pub(crate) fn put(&self, key: VerifierCacheKey) {
        if verifier_cache_enabled() {
            let mut cache = self.0.lock();
            cache.put(key, ());
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
