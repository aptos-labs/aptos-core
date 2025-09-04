// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    api_index::ApiIndexProvider, metrics::MetricsProvider, noise::NoiseProvider,
    system_information::SystemInformationProvider,
};
use std::sync::Arc;

/// This struct is a container for all the different providers that we have.
/// It is used to provide a single point of access to all Providers that can
/// then be passed into Checkers.
///
/// When a new Provider is added, it must be added to this struct. The same
/// Provider can be added multiple times for different sources. It is expected
/// that at the start of handling a request, this struct will be built with
/// whatever individual Provider instances can be created based on the request.
///
/// Alternative designs were considered, such as having each Checker define in
/// its type signature what Providers it needs, but that introduces complexities
/// of its own, namely more duplication in runtime checking. See this post for
/// more discussion on the topic: https://stackoverflow.com/questions/74723985.
///
/// You'll notice that some of these Providers are wrapped in an Arc. We do this
/// for Providers that could be used between requests, such Providers created for
/// querying the baseline node. Providers that are only used for a single request
/// are not wrapped in an Arc since they're only used once.
#[derive(Clone, Debug, Default)]
pub struct ProviderCollection {
    /// Provider that returns the information from the / endpoint of the API.
    pub baseline_api_index_provider: Option<Arc<ApiIndexProvider>>,

    /// Provider that returns the information from the / endpoint of the API.
    pub target_api_index_provider: Option<Arc<ApiIndexProvider>>,

    /// Provider that returns a metrics scrape.
    pub baseline_metrics_provider: Option<Arc<MetricsProvider>>,

    /// Provider that returns a metrics scrape.
    pub target_metrics_provider: Option<MetricsProvider>,

    /// Provider that returns a metrics scrape.
    pub baseline_system_information_provider: Option<Arc<SystemInformationProvider>>,

    /// Provider that returns a metrics scrape.
    pub target_system_information_provider: Option<SystemInformationProvider>,

    /// Provider that wraps functionality for connecting to the node via noise.
    pub baseline_noise_provider: Option<Arc<NoiseProvider>>,

    /// Provider that wraps functionality for connecting to the node via noise.
    pub target_noise_provider: Option<NoiseProvider>,
}

impl ProviderCollection {
    pub fn new() -> Self {
        Self {
            baseline_api_index_provider: None,
            target_api_index_provider: None,
            baseline_metrics_provider: None,
            target_metrics_provider: None,
            baseline_system_information_provider: None,
            target_system_information_provider: None,
            baseline_noise_provider: None,
            target_noise_provider: None,
        }
    }
}
