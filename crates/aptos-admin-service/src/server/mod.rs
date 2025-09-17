// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{AdminServiceConfig, AuthenticationConfig, NodeConfig};
use aptos_consensus::{
    persistent_liveness_storage::StorageWriteProxy, quorum_store::quorum_store_db::QuorumStoreDB,
};
use aptos_infallible::RwLock;
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::DbReaderWriter;
use aptos_system_utils::utils::reply_with_status;
#[cfg(target_os = "linux")]
use aptos_system_utils::{
    profiling::handle_cpu_profiling_request, thread_dump::handle_thread_dump_request,
};
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
#[cfg(unix)]
mod malloc;
mod mempool;

#[derive(Default)]
pub struct Context {
    config: AdminServiceConfig,

    aptos_db: RwLock<Option<Arc<DbReaderWriter>>>,
    consensus_db: RwLock<Option<Arc<StorageWriteProxy>>>,
    quorum_store_db: RwLock<Option<Arc<QuorumStoreDB>>>,
    mempool_client_sender: RwLock<Option<MempoolClientSender>>,
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

    fn set_mempool_client_sender(&self, mempool_client_sender: MempoolClientSender) {
        *self.mempool_client_sender.write() = Some(mempool_client_sender);
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
        let config = node_config.admin_service.clone();
        // Fetch the service port and address
        let service_port = config.port;
        let service_address = config.address.clone();

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

        // TODO(grao): Consider support enabling the service through an authenticated request.
        let enabled = config.enabled.unwrap_or(false);
        let admin_service = Self {
            runtime,
            context: Arc::new(Context {
                config,
                ..Default::default()
            }),
        };
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

    pub fn set_mempool_client_sender(&self, mempool_client_sender: MempoolClientSender) {
        self.context
            .set_mempool_client_sender(mempool_client_sender)
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
        if context.config.authentication_configs.is_empty() {
            authenticated = true;
        } else {
            for authentication_config in &context.config.authentication_configs {
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
            (hyper::Method::GET, "/profilez") => handle_cpu_profiling_request(req).await,
            #[cfg(target_os = "linux")]
            (hyper::Method::GET, "/threadz") => handle_thread_dump_request(req).await,
            #[cfg(unix)]
            (hyper::Method::GET, "/malloc/stats") => {
                malloc::handle_malloc_stats_request(context.config.malloc_stats_max_len)
            },
            #[cfg(unix)]
            (hyper::Method::GET, "/malloc/dump_profile") => malloc::handle_dump_profile_request(),
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
            (hyper::Method::GET, "/debug/mempool/parking-lot/addresses") => {
                let mempool_client_sender = context.mempool_client_sender.read().clone();
                if mempool_client_sender.is_some() {
                    mempool::mempool_handle_parking_lot_address_request(
                        req,
                        mempool_client_sender.unwrap(),
                    )
                    .await
                } else {
                    Ok(reply_with_status(
                        StatusCode::NOT_FOUND,
                        "Mempool parking lot is not available.",
                    ))
                }
            },
            _ => Ok(reply_with_status(StatusCode::NOT_FOUND, "Not found.")),
        }
    }
}
