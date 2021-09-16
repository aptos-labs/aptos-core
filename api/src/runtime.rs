// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, filters};

use diem_config::config::ApiConfig;
use diem_types::chain_id::ChainId;
use storage_interface::MoveDbReader;

use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};

/// Creates HTTP server (warp-based)
/// Returns handle to corresponding Tokio runtime
pub fn bootstrap(chain_id: ChainId, db: Arc<dyn MoveDbReader>, config: &ApiConfig) -> Runtime {
    let runtime = Builder::new_multi_thread()
        .thread_name("api")
        .enable_all()
        .build()
        .expect("[api] failed to create runtime");

    let address = config.address;
    runtime.spawn(async move {
        let service = Context::new(chain_id, db);
        let routes = filters::routes(service);
        let server = warp::serve(routes).bind(address);
        server.await
    });
    runtime
}
