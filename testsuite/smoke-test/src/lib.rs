// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Defines Forge Tests
pub mod aptos;
pub mod fullnode;
pub mod indexer;
pub mod nft_transaction;
pub mod rest_api;
pub mod transaction;

// Converted to local Forge backend
#[cfg(test)]
mod client;
#[cfg(test)]
mod consensus;
#[cfg(test)]
mod full_nodes;
#[cfg(test)]
mod network;
#[cfg(test)]
mod operational_tooling;
#[cfg(test)]
mod state_sync;
#[cfg(test)]
mod state_sync_v2;
#[cfg(test)]
mod storage;

#[cfg(test)]
mod smoke_test_environment;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod workspace_builder;
