// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

extern crate core;

#[cfg(test)]
mod account_abstraction;
#[cfg(test)]
mod aptos;
#[cfg(test)]
mod aptos_cli;
#[cfg(test)]
mod client;
#[cfg(test)]
mod consensus;
#[cfg(test)]
mod consensus_key_rotation;
#[cfg(test)]
mod consensus_observer;
#[cfg(test)]
mod execution;
#[cfg(test)]
mod full_nodes;
#[cfg(test)]
mod fullnode;
#[cfg(test)]
mod genesis;
#[cfg(test)]
mod indexer;
#[cfg(test)]
mod inspection_service;
#[cfg(test)]
mod jwks;
#[cfg(test)]
mod keyless;
#[cfg(test)]
mod network;
#[cfg(test)]
mod permissioned_delegation;
#[cfg(test)]
mod randomness;
#[cfg(test)]
mod rest_api;
#[cfg(test)]
mod rosetta;
#[cfg(test)]
mod state_sync;
#[cfg(test)]
mod state_sync_utils;
#[cfg(test)]
mod storage;
#[cfg(test)]
mod test_smoke_tests;
#[cfg(test)]
mod transaction;
#[cfg(test)]
mod txn_broadcast;
#[cfg(test)]
mod txn_emitter;
#[cfg(test)]
mod upgrade;

#[cfg(test)]
mod smoke_test_environment;

#[cfg(test)]
mod utils;

#[cfg(test)]
mod validator_txns;

#[cfg(test)]
mod execution_pool;
#[cfg(test)]
mod workspace_builder;
