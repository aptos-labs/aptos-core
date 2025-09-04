// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::ProviderError;
use futures::Future;
use std::{
    fmt::Debug,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// This struct helps with caching of Provider output.
#[derive(Debug)]
pub struct OutputCache<T: Clone + Debug> {
    /// The cache TTL.
    pub cache_ttl: Duration,
    /// The last time the Provider was run.
    pub last_run: RwLock<Instant>,
    /// The output of the last run of the Provider.
    pub last_output: RwLock<Option<T>>,
}

impl<T: Clone + Debug> OutputCache<T> {
    /// Create a new OutputCache.
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            cache_ttl,
            last_run: RwLock::new(Instant::now()),
            last_output: RwLock::new(None),
        }
    }

    /// Get the output of the Provider, either from the cache or by running the
    /// Provider.
    pub async fn get(
        &self,
        func: impl Future<Output = Result<T, ProviderError>>,
    ) -> Result<T, ProviderError> {
        // If the cache isn't too old and there is a value, return it.
        if self.last_run.read().await.elapsed() < self.cache_ttl {
            if let Some(last_output) = &*self.last_output.read().await {
                return Ok(last_output.clone());
            }
        }

        // Otherwise fetch the value and update the cache. We take the locks while
        // fetching the new value so we don't waste effort fetching it multiple times.
        let mut last_output = self.last_output.write().await;
        let mut last_run = self.last_run.write().await;
        let new_output = func.await?;
        *last_output = Some(new_output.clone());
        *last_run = Instant::now();
        Ok(new_output)
    }
}
