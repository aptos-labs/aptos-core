// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    cache::OutputCache,
    traits::{Provider, ProviderError},
    CommonProviderConfig,
};
use anyhow::Result;
use velor_rest_client::{velor_api_types::IndexResponse, Client};
use async_trait::async_trait;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApiIndexProviderConfig {
    #[serde(default, flatten)]
    pub common: CommonProviderConfig,
}

// TODO: Add cache, make it configurable.
#[derive(Clone, Debug)]
pub struct ApiIndexProvider {
    pub config: ApiIndexProviderConfig,

    // We make this public to allow convenient (but ideally avoidable) backdoor
    // access to the client for use cases where the memoization / retrying support
    // offered by a Provider isn't relevant.
    pub client: Client,

    // This output cache helps prevent the Provider from overfetching the data within
    // a short window of time. Downstream Checkers should be aware of this behaviour.
    output_cache: Arc<OutputCache<IndexResponse>>,
}

impl ApiIndexProvider {
    pub fn new(config: ApiIndexProviderConfig, client: Client) -> Self {
        let output_cache = Arc::new(OutputCache::new(Duration::from_millis(
            config.common.cache_ttl_ms,
        )));
        Self {
            config,
            client,
            output_cache,
        }
    }
}

#[async_trait]
impl Provider for ApiIndexProvider {
    type Output = IndexResponse;

    async fn provide(&self) -> Result<Self::Output, ProviderError> {
        self.output_cache
            .get(
                self.client
                    .get_index()
                    .map_ok(|r| r.into_inner())
                    .map_err(|e| ProviderError::RetryableEndpointError("/", e.into())),
            )
            .await
    }

    fn explanation() -> &'static str {
        "The API port was not included in the request."
    }
}
