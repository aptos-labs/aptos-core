// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod accounts_test;
mod blocks_test;
mod converter_test;
mod events_test;
mod index_test;
mod invalid_post_request_test;
mod modules;
mod multisig_transactions_test;
mod objects;
mod resource_groups;
mod secp256k1_ecdsa;
mod simulation_test;
mod state_test;
mod string_resource_test;
mod transaction_vector_test;
mod transactions_test;
mod view_function;
mod webauthn_secp256r1_ecdsa;

use aptos_api_test_context::{new_test_context as super_new_test_context, TestContext};
use aptos_config::config::{internal_indexer_db_config::InternalIndexerDBConfig, NodeConfig};

fn new_test_context(test_name: String) -> TestContext {
    new_test_context_with_config(test_name, NodeConfig::default())
}

fn new_test_context_with_config(test_name: String, node_config: NodeConfig) -> TestContext {
    super_new_test_context(test_name, node_config, false)
}

fn new_test_context_with_db_sharding_and_internal_indexer(test_name: String) -> TestContext {
    let mut node_config = NodeConfig::default();
    node_config.storage.rocksdb_configs.enable_storage_sharding = true;
    node_config.indexer_db_config = InternalIndexerDBConfig::new(true, true, true, 10_000);
    super_new_test_context(test_name, node_config, true)
}
