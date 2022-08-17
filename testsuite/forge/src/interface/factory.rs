// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{GenesisConfig, Swarm, Version};
use crate::{GenesisConfigFn, NodeConfigFn, Result};
use rand::rngs::StdRng;
use std::num::NonZeroUsize;
use std::time::Duration;

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
    ) -> Result<Box<dyn Swarm>>;
}
