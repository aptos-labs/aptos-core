// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use cfg_if::cfg_if;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::num::NonZeroUsize;

/// Cache key for verified modules.
///
/// The key has two components: the module hash, and a digest of the verifier
/// configuration that produced the verification result. Reusing a verified
/// entry under a different verifier configuration is unsound — a module
/// admitted under a permissive config may not be admitted under a stricter
/// one — so the configuration digest must participate in the key.
type Key = ([u8; 32], [u8; 32]);

/// Cache for already verified modules. Since loader V1 uses such a cache to not perform repeated
/// verifications, possibly even across blocks, for comparative performance we need to have it as
/// well. For now, we keep it as a separate cache to make sure there is no interference between V1
/// and V2 implementations.
pub(crate) struct VerifiedModuleCache(Mutex<lru::LruCache<Key, ()>>);

impl VerifiedModuleCache {
    /// Maximum size of the cache. When modules are cached, they can skip re-verification.
    const VERIFIED_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(100_000).unwrap();

    /// Returns new empty verified module cache.
    pub(crate) fn empty() -> Self {
        Self(Mutex::new(lru::LruCache::new(Self::VERIFIED_CACHE_SIZE)))
    }

    /// Returns true if the (module hash, verifier config digest) pair is contained in the cache.
    pub(crate) fn contains(
        &self,
        module_hash: &[u8; 32],
        verifier_config_digest: &[u8; 32],
    ) -> bool {
        // Note: need to use get to update LRU queue.
        verifier_cache_enabled()
            && self
                .0
                .lock()
                .get(&(*module_hash, *verifier_config_digest))
                .is_some()
    }

    /// Inserts the (module hash, verifier config digest) pair into the cache, marking the
    /// corresponding module as locally verified under that configuration.
    pub(crate) fn put(&self, module_hash: [u8; 32], verifier_config_digest: [u8; 32]) {
        if verifier_cache_enabled() {
            let mut cache = self.0.lock();
            cache.put((module_hash, verifier_config_digest), ());
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
            !cfg!(test) && !cfg!(feature = "testing")
        }
    }
}
