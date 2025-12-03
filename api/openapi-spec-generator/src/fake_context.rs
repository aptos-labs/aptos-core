// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_mempool::mocks::MockSharedMempool;
use aptos_storage_interface::mock::MockDbReaderWriter;
use aptos_types::chain_id::ChainId;
use std::sync::Arc;

// This is necessary for building the API with how the code is structured currently.
pub fn get_fake_context() -> Context {
    let mempool = MockSharedMempool::new_with_runtime();
    Context::new(
        ChainId::test(),
        Arc::new(MockDbReaderWriter),
        mempool.ac_client,
        NodeConfig::default(),
        None, /* table info reader */
    )
}
