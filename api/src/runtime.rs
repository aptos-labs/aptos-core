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

    let service = Arc::new(Context::new(chain_id, db));
    let routes = filters::routes(service);

    // Ensure that we actually bind to the socket first before spawning the
    // server tasks. This helps in tests to prevent races where a client attempts
    // to make a request before the server task is actually listening on the
    // socket.
    //
    // Note: we need to enter the runtime context first to actually bind, since
    //       tokio TcpListener can only be bound inside a tokio context.

    let _guard = runtime.enter();

    let server = warp::serve(routes).bind(config.address);
    runtime.handle().spawn(server);
    runtime
}
