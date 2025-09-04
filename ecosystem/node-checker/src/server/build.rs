// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    checker::build_checkers,
    configuration::{read_configuration_from_file, BaselineConfiguration},
    provider::{
        api_index::ApiIndexProvider, metrics::MetricsProvider, noise::NoiseProvider,
        system_information::SystemInformationProvider, Provider, ProviderCollection,
        MISSING_PROVIDER_MESSAGE,
    },
    runner::{Runner, SyncRunner},
};
use anyhow::{bail, Context, Result};
use velor_logger::info;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

/// This struct is a wrapper to help with all the different baseline
/// node configurations. The key here is the configuration_id.
#[derive(Debug)]
pub struct BaselineConfigurationRunners<R: Runner>(
    pub HashMap<String, BaselineConfigurationRunner<R>>,
);

/// Just a baseline configuration plus the relevant Handler.
#[derive(Debug)]
pub struct BaselineConfigurationRunner<R: Runner> {
    pub configuration: BaselineConfiguration,
    pub runner: R,
}

/// Build BaselineConfigurationRunners from the baseline configs.
pub async fn build_baseline_configuration_runners(
    baseline_node_config_paths: &[PathBuf],
) -> Result<BaselineConfigurationRunners<SyncRunner>> {
    let mut baseline_configuration_runners = HashMap::new();
    for path in baseline_node_config_paths.iter() {
        info!("Building baseline configuration from {}", path.display());
        let cfg = read_configuration_from_file(path.to_path_buf())
            .with_context(|| format!("Failed to read configuration from {}", path.display()))?;
        let id = cfg.configuration_id.clone();
        let bcr = build_baseline_configuration_runner(cfg).await?;
        info!(
            "Successfully built baseline configuration from {}: {}",
            path.display(),
            id
        );
        baseline_configuration_runners.insert(id, bcr);
    }
    Ok(BaselineConfigurationRunners(baseline_configuration_runners))
}

/// Given a baseline configuration, return a BaselineConfigurationRunner, which
/// is a wrapper around the baseline configuration and its Runner, which contains
/// all the Checkers, Providers, etc. we built based on the configuration.
///
/// In this function, we attempt to build Providers that operate against the baseline
/// node. If the configuration specifies some information that means we can build a
/// Provider, e.g. they provide a noise port, we will try to build that Provider, in
/// this example a NoiseProvider. If we can't build it / the `provide` call fails, we
/// throw an error that will ultimately result in startup failure. In short, we choose
/// to fail at startup if a baseline Provider that should be working is not working.
async fn build_baseline_configuration_runner(
    configuration: BaselineConfiguration,
) -> Result<BaselineConfigurationRunner<SyncRunner>> {
    // Build Checkers based on the baseline configuration.
    let checkers = build_checkers(&configuration.checkers).context("Failed to build Checkers")?;

    // Build Providers that support being built based on the baseline configuration.
    let mut provider_collection = ProviderCollection::new();

    if let Some(node_address) = &configuration.node_address {
        let api_client = node_address
            .get_api_client(std::time::Duration::from_secs(4))
            .ok();
        let metrics_client = node_address
            .get_metrics_client(std::time::Duration::from_secs(4))
            .ok();

        // Build a MetricsProvider for the baseline. Confirm it works against
        // that baseline node (i.e. make sure the metrics endpoint is accessible).
        if let Some(metrics_client) = metrics_client {
            let metrics_client = Arc::new(metrics_client);
            let metrics_provider = MetricsProvider::new(
                configuration.provider_configs.metrics.clone(),
                metrics_client.clone(),
                node_address.url.clone(),
                node_address.get_metrics_port().unwrap(),
            );
            metrics_provider.provide().await.context(format!(
                "Failed to build MetricsProvider for baseline configuration {}, ensure the /metrics endpoint is accessible",
                configuration.configuration_id
            ))?;
            info!(
                "Successfully built MetricsProvider for baseline configuration {}",
                configuration.configuration_id
            );
            provider_collection.baseline_metrics_provider = Some(Arc::new(metrics_provider));

            // Also try to build the SystemInformationProvider.
            let system_information_provider = SystemInformationProvider::new(
                configuration.provider_configs.system_information.clone(),
                metrics_client,
                node_address.url.clone(),
                node_address.get_metrics_port().unwrap(),
            );
            system_information_provider.provide().await.context(format!(
                "Failed to build SystemInformationProvider for baseline configuration {}, ensure the /system_information endpoint is accessible",
                configuration.configuration_id
            ))?;
            info!(
                "Successfully built SystemInformationProvider for baseline configuration {}",
                configuration.configuration_id
            );
            provider_collection.baseline_system_information_provider =
                Some(Arc::new(system_information_provider));
        }

        // Build an ApiIndexProvider for the baseline. Confirm it works against
        // that baseline node (e.g. make sure the / API endpoint is accessible).
        if let Some(api_client) = api_client {
            let api_index_provider = Arc::new(ApiIndexProvider::new(
                configuration.provider_configs.api_index.clone(),
                api_client,
            ));
            api_index_provider.provide().await.context(format!(
                "Failed to build ApiProvider for baseline configuration {}, ensure the API is accessible",
                configuration.configuration_id
            ))?;
            info!(
                "Successfully built ApiIndexProvider for baseline configuration {}",
                configuration.configuration_id
            );
            provider_collection.baseline_api_index_provider = Some(api_index_provider.clone());

            // If we have an ApiIndexProvider and a noise port / public key, we can try to create a NoiseProvider.
            if node_address.get_noise_port().is_some() || node_address.get_public_key().is_some() {
                let noise_network_address = node_address
                    .as_noise_network_address()
                    .context(format!(
                        "Failed to build NoiseProvider for baseline configuration {}, ensure the noise port is accessible",
                        configuration.configuration_id
                    ))?;
                let noise_provider = NoiseProvider::new(
                    configuration.provider_configs.noise.clone(),
                    noise_network_address,
                    api_index_provider,
                );
                // Specifically check that we can establish a connection with the baseline node on the noise port.
                noise_provider.establish_connection().await.context(format!(
                    "Failed to build NoiseProvider for baseline configuration {}, ensure a noise port and public key were provided and they're both valid",
                    configuration.configuration_id
                ))?;
                provider_collection.baseline_noise_provider = Some(Arc::new(noise_provider));
            }
        }
    }

    let runner = SyncRunner::new(
        configuration.runner_config.clone(),
        provider_collection,
        configuration.provider_configs.clone(),
        checkers,
    );

    // Finally, if a baseline was provided in the config, we run the runner against
    // the baseline itself. If the configuration listed a Checker that requires a
    // particular Provider, this will fail if that Provider was not created. If we
    // don't do this now, it'll fail later when NHC handles requests. With this run
    // we're only looking for hard errors or check results that appeared to fail
    // because of a missing Provider.
    if let Some(node_address) = &configuration.node_address {
        let message = format!(
            "Failed to run the Checker suite against the baseline node itself \
                for {}. This implies that a Checker has been enabled in the baseline \
                config but the necessary baseline information was not provided. For \
                example, this error might happen if the NodeIdentityChecker was enabled \
                but the API port for the baseline was not provided, since that checker \
                needs to be able to query the API of the baseline node. ",
            configuration.configuration_id
        );
        let results = runner
            .run(node_address)
            .await
            .with_context(|| message.clone())?;
        let mut missing_provider_results = Vec::new();
        for result in results.check_results.into_iter() {
            if result.score < 100 && result.headline.ends_with(MISSING_PROVIDER_MESSAGE) {
                missing_provider_results.push(result);
            }
        }
        if !missing_provider_results.is_empty() {
            bail!(
                "{} The following check results should explain the problem: {:#?}",
                message,
                missing_provider_results
            );
        }
    }

    Ok(BaselineConfigurationRunner {
        configuration,
        runner,
    })
}
