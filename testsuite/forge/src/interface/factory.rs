// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::{GenesisConfig, Swarm, Version};
use crate::Result;
use rand::rngs::StdRng;
use std::num::NonZeroUsize;

/// Trait used to represent a interface for constructing a launching new networks
#[async_trait::async_trait]
pub trait Factory {
    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a>;

    async fn launch_swarm(
        &self,
        rng: &mut StdRng,
        node_num: NonZeroUsize,
        version: &Version,
        genesis_version: &Version,
        genesis_modules: Option<&GenesisConfig>,
    ) -> Result<Box<dyn Swarm>>;
}
