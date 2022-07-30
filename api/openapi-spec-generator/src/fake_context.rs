// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_mempool::mocks::MockSharedMempool;
use aptos_types::chain_id::ChainId;
use std::sync::Arc;
use storage_interface::mock::MockDbReaderWriter;

// This is necessary for building the API with how the code is structured currently.
pub fn get_fake_context() -> Context {
    let mempool = MockSharedMempool::new();
    Context::new(
        ChainId::test(),
        Arc::new(MockDbReaderWriter),
        mempool.ac_client,
        NodeConfig::default(),
    )
}
