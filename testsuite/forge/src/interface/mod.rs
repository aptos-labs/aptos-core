// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod admin;
pub use admin::*;
mod aptos;
pub use self::aptos::*;
mod network;
pub use network::*;
mod test;
pub use test::*;
mod factory;
pub use factory::*;
mod swarm;
pub use swarm::*;
mod chaos;
pub use chaos::*;
mod node;
pub use node::*;
mod chain_info;
pub mod system_metrics;

pub use chain_info::*;
use framework::ReleaseBundle;

/// A wrapper around a usize in order to represent an opaque version of a Node.
///
/// It is intended that backends will be able to take this opaque version identifier and lookup the
/// appropriate version information internally to be able to determine the version of node software
/// to use.
///
/// It's expected that `Version`s returned by querying a `Factory` or a `Swarm` will be sort-able
/// such that they'll be ordered with older versions first, e.g. older -> newer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(usize, String);

impl Version {
    pub fn new(version: usize, display_string: String) -> Self {
        Self(version, display_string)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

#[derive(Clone)]
pub enum GenesisConfig {
    Bundle(ReleaseBundle),
    Path(String),
}
