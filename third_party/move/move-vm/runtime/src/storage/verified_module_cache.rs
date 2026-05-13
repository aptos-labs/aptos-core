// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use lazy_static::lazy_static;
use parking_lot::Mutex;

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

/// In test builds the cache is treated as empty: lookups always miss and inserts are no-ops.
/// This keeps unit tests deterministic regardless of the process-global LRU's accumulated state.
const fn cache_active() -> bool {
    !cfg!(test) && !cfg!(feature = "testing")
}

impl VerifiedModuleCache {
    /// Maximum size of the cache. When modules are cached, they can skip re-verification.
    const VERIFIED_CACHE_SIZE: usize = 100_000;

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
        cache_active() && self.0.lock().contains(&(*module_hash, *verifier_config_digest))
    }

    /// Inserts the (module hash, verifier config digest) pair into the cache, marking the
    /// corresponding module as locally verified under that configuration.
    pub(crate) fn put(&self, module_hash: [u8; 32], verifier_config_digest: [u8; 32]) {
        if cache_active() {
            self.0
                .lock()
                .put((module_hash, verifier_config_digest), ());
        }
    }
}

lazy_static! {
    pub(crate) static ref VERIFIED_MODULES_V2: VerifiedModuleCache = VerifiedModuleCache::empty();
}
