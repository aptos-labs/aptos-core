// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod account_abstraction_test;
mod accounts_test;
mod blocks_test;
mod converter_test;
mod event_v2_translation_test;
mod events_test;
mod function_value_test;
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

use aptos_api_test_context::{new_test_context_inner as super_new_test_context, TestContext};
use aptos_config::config::{internal_indexer_db_config::InternalIndexerDBConfig, NodeConfig};

fn new_test_context(test_name: String) -> TestContext {
    new_test_context_with_config(test_name, NodeConfig::default(), false, false)
}

fn new_test_context_with_config(
    test_name: String,
    node_config: NodeConfig,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TestContext {
    super_new_test_context(
        test_name,
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
}

fn new_test_context_with_orderless_flags(
    test_name: String,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TestContext {
    super_new_test_context(
        test_name,
        NodeConfig::default(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
}
