// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_config::config::NodeConfig;
use velor_mempool::MempoolClientSender;
use velor_storage_interface::DbReader;
use velor_types::chain_id::ChainId;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[cfg(feature = "indexer")]
pub fn bootstrap_indexer(
    node_config: &NodeConfig,
    chain_id: ChainId,
    velor_db: Arc<dyn DbReader>,
    mp_client_sender: MempoolClientSender,
) -> Result<Option<Runtime>, anyhow::Error> {
    use velor_indexer::runtime::bootstrap as bootstrap_indexer_stream;

    match bootstrap_indexer_stream(&node_config, chain_id, velor_db, mp_client_sender) {
        None => Ok(None),
        Some(res) => res.map(Some),
    }
}

#[cfg(not(feature = "indexer"))]
pub fn bootstrap_indexer(
    _node_config: &NodeConfig,
    _chain_id: ChainId,
    _velor_db: Arc<dyn DbReader>,
    _mp_client_sender: MempoolClientSender,
) -> Result<Option<Runtime>, anyhow::Error> {
    Ok(None)
}
