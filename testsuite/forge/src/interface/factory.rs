// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{GenesisConfig, Swarm, Version};
use crate::{GenesisConfigFn, NodeConfigFn, OverrideNodeConfigFn, Result};
use rand::rngs::StdRng;
use std::{num::NonZeroUsize, time::Duration};

/// Trait used to represent a interface for constructing a launching new networks
#[async_trait::async_trait]
pub trait Factory {
    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a>;

    async fn launch_swarm(
        &self,
        rng: &mut StdRng,
        num_validators: NonZeroUsize,
        num_fullnodes: usize,
        version: &Version,
        genesis_version: &Version,
        genesis_modules: Option<&GenesisConfig>,
        cleanup_duration: Duration,
        genesis_config_fn: Option<GenesisConfigFn>,
        node_config_fn: Option<NodeConfigFn>,
        existing_db_tag: Option<String>,
        override_node_config_fn: Option<OverrideNodeConfigFn>,
    ) -> Result<Box<dyn Swarm>>;
}
