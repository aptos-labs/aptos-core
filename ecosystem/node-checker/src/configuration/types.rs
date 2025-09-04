// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::NodeAddress;
use crate::{checker::CheckerConfig, provider::ProviderConfigs, runner::SyncRunnerConfig};
use serde::{Deserialize, Serialize};

/// This defines a single baseline configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineConfiguration {
    /// The address of the baseline node to use for this configuration. This is
    /// only necessary if this baseline configuration uses a Checker that
    /// requires information from a baseline node to operate.
    pub node_address: Option<NodeAddress>,

    /// This is the ID we expect clients to send over the wire to select
    /// which configuration they want to use. e.g. devnet_fullnode
    pub configuration_id: String,

    /// This is the name we will show for this configuration to users.
    /// For example, if someone opens the NHC frontend, they will see this name
    /// in a dropdown list of configurations they can test their node against.
    /// e.g. "Devnet Fullnode", "Testnet Validator", etc.
    pub configuration_name: String,

    /// Config for the runner.
    #[serde(default)]
    pub runner_config: SyncRunnerConfig,

    /// Configs for specific Providers.
    #[serde(default)]
    pub provider_configs: ProviderConfigs,

    /// Configs for the checkers to use.
    pub checkers: Vec<CheckerConfig>,
}
