// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_infallible::RwLock;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use request_handler::RequestHandler;
use std::{convert::Infallible, mem, net::SocketAddr, sync::Arc};
use tokio::{
    sync::{
        oneshot,
        oneshot::{Receiver, Sender},
    },
    task::JoinHandle,
};

pub(crate) mod request_handler;

/// A dummy OIDC provider.
pub struct DummyHttpServer {
    close_tx: Sender<()>,
    url: String,
    handler_holder: Arc<RwLock<Option<Arc<dyn RequestHandler>>>>,
    server_join_handle: JoinHandle<()>,
}

impl DummyHttpServer {
    pub(crate) async fn spawn() -> Self {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let handler_holder = Arc::new(RwLock::new(None));
        let (port_tx, port_rx) = oneshot::channel::<u16>();
        let (close_tx, close_rx) = oneshot::channel::<()>();
        let server_join_handle = tokio::spawn(Self::run_server(
            addr,
            handler_holder.clone(),
            port_tx,
            close_rx,
        ));
        let actual_port = port_rx.await.unwrap();
        let url = format!("http://127.0.0.1:{}", actual_port);
        Self {
            close_tx,
            url,
            handler_holder,
            server_join_handle,
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn update_request_handler(
        &self,
        handler: Option<Arc<dyn RequestHandler>>,
    ) -> Option<Arc<dyn RequestHandler>> {
        mem::replace(&mut *self.handler_holder.write(), handler)
    }

    pub async fn shutdown(self) {
        let DummyHttpServer {
            close_tx,
            server_join_handle,
            ..
        } = self;
        close_tx.send(()).unwrap();
        server_join_handle.await.unwrap();
    }
}

// Private functions.
impl DummyHttpServer {
    async fn run_server(
        addr: SocketAddr,
        handler_holder: Arc<RwLock<Option<Arc<dyn RequestHandler>>>>,
        port_tx: Sender<u16>,
        close_rx: Receiver<()>,
    ) {
        let make_svc = make_service_fn(move |_| {
            let handler_holder_clone = handler_holder.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    Self::handle_request(req, handler_holder_clone.clone())
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);
        let actual_addr = server.local_addr();
        port_tx.send(actual_addr.port()).unwrap();

        // Graceful shutdown
        let graceful = server.with_graceful_shutdown(async {
            close_rx.await.unwrap();
        });

        graceful.await.unwrap();
    }

    async fn handle_request(
        request: Request<Body>,
        handler_holder: Arc<RwLock<Option<Arc<dyn RequestHandler>>>>,
    ) -> Result<Response<Body>, Infallible> {
        let handler = handler_holder.write();
        let raw_response = handler.as_ref().unwrap().handle(request);
        Ok(Response::new(Body::from(raw_response)))
    }
}
