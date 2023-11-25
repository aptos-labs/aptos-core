// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::reply_with_status;
use aptos_config::config::{AuthenticationConfig, NodeConfig};
use aptos_consensus::{
    persistent_liveness_storage::StorageWriteProxy, quorum_store::quorum_store_db::QuorumStoreDB,
};
use aptos_infallible::RwLock;
use aptos_logger::info;
use aptos_storage_interface::DbReaderWriter;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use std::{
    collections::HashMap,
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};
use tokio::runtime::Runtime;

mod consensus;
#[cfg(target_os = "linux")]
mod profiling;
#[cfg(target_os = "linux")]
mod thread_dump;
mod utils;

#[derive(Default)]
pub struct Context {
    authentication_configs: Vec<AuthenticationConfig>,

    aptos_db: RwLock<Option<Arc<DbReaderWriter>>>,
    consensus_db: RwLock<Option<Arc<StorageWriteProxy>>>,
    quorum_store_db: RwLock<Option<Arc<QuorumStoreDB>>>,
}

impl Context {
    fn set_aptos_db(&self, aptos_db: Arc<DbReaderWriter>) {
        *self.aptos_db.write() = Some(aptos_db);
    }

    fn set_consensus_dbs(
        &self,
        consensus_db: Arc<StorageWriteProxy>,
        quorum_store_db: Arc<QuorumStoreDB>,
    ) {
        *self.consensus_db.write() = Some(consensus_db);
        *self.quorum_store_db.write() = Some(quorum_store_db);
    }
}

pub struct AdminService {
    runtime: Runtime,
    context: Arc<Context>,
}

impl AdminService {
    /// Starts the admin service that listens on the configured address and handles various endpoint
    /// requests.
    pub fn new(node_config: &NodeConfig) -> Self {
        // Fetch the service port and address
        let service_port = node_config.admin_service.port;
        let service_address = node_config.admin_service.address.clone();

        // Create the admin service socket address
        let address: SocketAddr = (service_address.as_str(), service_port)
            .to_socket_addrs()
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to parse {}:{} as address",
                    service_address, service_port
                )
            })
            .next()
            .unwrap();

        // Create a runtime for the admin service
        let runtime = aptos_runtimes::spawn_named_runtime("admin".into(), None);

        let admin_service = Self {
            runtime,
            context: Arc::new(Context {
                authentication_configs: node_config.admin_service.authentication_configs.clone(),
                ..Default::default()
            }),
        };

        // TODO(grao): Consider support enabling the service through an authenticated request.
        let enabled = node_config.admin_service.enabled.unwrap_or(false);
        admin_service.start(address, enabled);

        admin_service
    }

    pub fn set_aptos_db(&self, aptos_db: Arc<DbReaderWriter>) {
        self.context.set_aptos_db(aptos_db)
    }

    pub fn set_consensus_dbs(
        &self,
        consensus_db: Arc<StorageWriteProxy>,
        quorum_store_db: Arc<QuorumStoreDB>,
    ) {
        self.context
            .set_consensus_dbs(consensus_db, quorum_store_db)
    }

    fn start(&self, address: SocketAddr, enabled: bool) {
        let context = self.context.clone();
        self.runtime.spawn(async move {
            let make_service = make_service_fn(move |_conn| {
                let context = context.clone();
                async move {
                    Ok::<_, Infallible>(service_fn(move |req| {
                        Self::serve_requests(context.clone(), req, enabled)
                    }))
                }
            });

            let server = Server::bind(&address).serve(make_service);
            info!("Started AdminService at {address:?}, enabled: {enabled}.");
            server.await
        });
    }

    async fn serve_requests(
        context: Arc<Context>,
        req: Request<Body>,
        enabled: bool,
    ) -> hyper::Result<Response<Body>> {
        if !enabled {
            return Ok(reply_with_status(
                StatusCode::NOT_FOUND,
                "AdminService is not enabled.",
            ));
        }

        let mut authenticated = false;
        if context.authentication_configs.is_empty() {
            authenticated = true;
        } else {
            for authentication_config in &context.authentication_configs {
                match authentication_config {
                    AuthenticationConfig::PasscodeSha256(passcode_sha256) => {
                        let query = req.uri().query().unwrap_or("");
                        let query_pairs: HashMap<_, _> =
                            url::form_urlencoded::parse(query.as_bytes()).collect();
                        let passcode: Option<String> =
                            query_pairs.get("passcode").map(|p| p.to_string());
                        if let Some(passcode) = passcode {
                            if sha256::digest(passcode) == *passcode_sha256 {
                                authenticated = true;
                            }
                        }
                    },
                }
            }
        };

        if !authenticated {
            return Ok(reply_with_status(
                StatusCode::NETWORK_AUTHENTICATION_REQUIRED,
                format!("{} endpoint requires authentication.", req.uri().path()),
            ));
        }

        match (req.method().clone(), req.uri().path()) {
            #[cfg(target_os = "linux")]
            (hyper::Method::GET, "/profilez") => profiling::handle_cpu_profiling_request(req).await,
            #[cfg(target_os = "linux")]
            (hyper::Method::GET, "/threadz") => thread_dump::handle_thread_dump_request(req).await,
            (hyper::Method::GET, "/debug/consensus/consensusdb") => {
                let consensus_db = context.consensus_db.read().clone();
                if let Some(consensus_db) = consensus_db {
                    consensus::handle_dump_consensus_db_request(req, consensus_db).await
                } else {
                    Ok(reply_with_status(
                        StatusCode::NOT_FOUND,
                        "Consensus db is not available.",
                    ))
                }
            },
            (hyper::Method::GET, "/debug/consensus/quorumstoredb") => {
                let quorum_store_db = context.quorum_store_db.read().clone();
                if let Some(quorum_store_db) = quorum_store_db {
                    consensus::handle_dump_quorum_store_db_request(req, quorum_store_db).await
                } else {
                    Ok(reply_with_status(
                        StatusCode::NOT_FOUND,
                        "Quorum store db is not available.",
                    ))
                }
            },
            (hyper::Method::GET, "/debug/consensus/block") => {
                let consensus_db = context.consensus_db.read().clone();
                let quorum_store_db = context.quorum_store_db.read().clone();
                if consensus_db.is_some() && quorum_store_db.is_some() {
                    consensus::handle_dump_block_request(
                        req,
                        consensus_db.unwrap(),
                        quorum_store_db.unwrap(),
                    )
                    .await
                } else {
                    Ok(reply_with_status(
                        StatusCode::NOT_FOUND,
                        "Consensus db and/or quorum store db is not available.",
                    ))
                }
            },
            _ => Ok(reply_with_status(StatusCode::NOT_FOUND, "Not found.")),
        }
    }
}
