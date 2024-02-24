use crate::dag::{
    dag_network::RpcResultWithResponder,
    observability::logging::{LogEvent, LogSchema},
    payload::store::DagPayloadStore,
    types::{BatchRequest, BatchResponse},
    DAGRpcResult, RpcHandler, RpcWithFallback, TDAGNetworkSender,
};
use aptos_config::config::DagFetcherConfig;
use aptos_consensus_types::{
    common::Author,
    dag_batch::{BatchDigest, DagBatch, DagBatchInfo},
};
use aptos_logger::{debug, error, info};
use aptos_time_service::TimeService;
use aptos_types::epoch_state::EpochState;
use async_trait::async_trait;
use futures::{
    stream::{AbortHandle, Abortable, Aborted, FuturesUnordered},
    Future, FutureExt, StreamExt,
};
use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
};

enum FetchServiceCommand {
    Request {
        request: BatchRequest,
        res_tx: oneshot::Sender<DagBatch>,
        responders: Vec<Author>,
    },
    CancelFetch(DagBatchInfo),
}

pub struct PayloadRequester {
    command_tx: Sender<FetchServiceCommand>,
}

impl PayloadRequester {
    pub fn request(
        &self,
        metadata: DagBatchInfo,
        responders: Vec<Author>,
    ) -> anyhow::Result<oneshot::Receiver<DagBatch>> {
        let (res_tx, res_rx) = oneshot::channel();
        let request = FetchServiceCommand::Request {
            request: BatchRequest::from(metadata),
            res_tx,
            responders,
        };
        self.command_tx
            .try_send(request)
            .map_err(|e| anyhow::anyhow!("unable to send request {}", e))?;
        Ok(res_rx)
    }

    pub fn cancel(&self, metadata: DagBatchInfo) -> anyhow::Result<()> {
        self.command_tx
            .try_send(FetchServiceCommand::CancelFetch(metadata))
            .map_err(|_| anyhow::anyhow!("unable to send cancel"))?;
        Ok(())
    }
}

pub struct PayloadFetcherService {
    inner: Arc<PayloadFetcher>,
    payload_store: Arc<DagPayloadStore>,
    command_rx: Receiver<FetchServiceCommand>,
    futures: FuturesUnordered<Pin<Box<dyn Future<Output = Result<BatchDigest, Aborted>> + Send>>>,
    inprogress_reqs: HashMap<BatchDigest, AbortHandle>,
}

impl PayloadFetcherService {
    pub fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<dyn TDAGNetworkSender>,
        payload_store: Arc<DagPayloadStore>,
        time_service: TimeService,
        config: DagFetcherConfig,
    ) -> (Self, PayloadRequester) {
        let (command_tx, command_rx) = tokio::sync::mpsc::channel(100);
        (
            Self {
                inner: Arc::new(PayloadFetcher {
                    epoch_state,
                    network,
                    time_service,
                    config,
                }),
                payload_store,
                command_rx,
                futures: FuturesUnordered::new(),
                inprogress_reqs: HashMap::new(),
            },
            PayloadRequester { command_tx },
        )
    }

    pub async fn start(self) {
        let Self {
            inner,
            payload_store,
            mut command_rx,
            mut futures,
            mut inprogress_reqs,
        } = self;
        loop {
            select! {
                Some(result) = futures.next() => {
                    if let Ok(digest) = result {
                        inprogress_reqs.remove(&digest);
                    }
                },
                // TODO: configure limit concurrent futures
                Some(command) = command_rx.recv(), if futures.len() < 50 => {
                    match command {
                        FetchServiceCommand::Request{request, responders, res_tx} => {
                            let id = request.id();
                            let digest = *request.batch_digest();

                            if !payload_store.is_missing(id, &digest) {
                                debug!("payload already exists: {:?}", request);
                                continue;
                            }

                            if inprogress_reqs.contains_key(&digest) {
                                debug!("payload already requested {:?}", request);
                                continue;
                            }

                            debug!("fetching payload {:?}", request);

                            let fetcher = inner.clone();
                            let store = payload_store.clone();
                            let future = async move {
                                let digest = *request.batch_digest();
                                // Fetch forever until aborted
                                loop {
                                    let result = fetcher.fetch(request.clone(), responders.clone()).await;
                                    match result {
                                        Ok(payload) => {
                                            if let Err(err) = store.insert(payload.clone()) {
                                                debug!("error inserting fetched payload to store {:?}", err);
                                            }
                                            debug!("payload fetched {}", payload.id());
                                            if let Err(err) = res_tx.send(payload) {
                                                debug!("error sending response {:?}", err);
                                            }
                                            return digest;
                                        },
                                        Err(e) => error!("unable to fetch {:?}", e),
                                    };
                                }
                            };
                            let (abort_handle, abort_registration) = AbortHandle::new_pair();
                            let abortable_fut = Abortable::new(future, abort_registration).boxed();
                            inprogress_reqs.insert(digest, abort_handle);
                            futures.push(abortable_fut);
                        },
                        FetchServiceCommand::CancelFetch(metadata) => {
                            debug!("cancel fetch payload {:?}", metadata);
                            if let Some(handle) = inprogress_reqs.remove(metadata.digest()) {
                                handle.abort();
                            }
                        },
                    }
                },
                else => {
                    debug!("stopping payload fetch service");
                    return;
                },
            }
        }
    }
}

pub struct PayloadFetcher {
    epoch_state: Arc<EpochState>,
    network: Arc<dyn TDAGNetworkSender>,
    time_service: TimeService,
    config: DagFetcherConfig,
}

impl PayloadFetcher {
    async fn fetch(
        &self,
        request: BatchRequest,
        responders: Vec<Author>,
    ) -> anyhow::Result<DagBatch> {
        debug!(LogSchema::new(LogEvent::FetchPayload), id = request.id(),);
        let mut rpc = RpcWithFallback::new(
            responders,
            request.clone().into(),
            Duration::from_millis(self.config.retry_interval_ms),
            Duration::from_millis(self.config.rpc_timeout_ms),
            self.network.clone(),
            self.time_service.clone(),
            self.config.min_concurrent_responders,
            self.config.max_concurrent_responders,
        );

        while let Some(RpcResultWithResponder { responder, result }) = rpc.next().await {
            match result {
                Ok(DAGRpcResult(Ok(response))) => {
                    match BatchResponse::try_from(response)
                        .and_then(|response| response.verify(&request, &self.epoch_state.verifier))
                    {
                        Ok(fetch_response) => {
                            return Ok(fetch_response.unwrap());
                        },
                        Err(err) => {
                            info!(error = ?err, "failure parsing/verifying fetch response from {}", responder);
                        },
                    };
                },
                Ok(DAGRpcResult(Err(dag_rpc_error))) => {
                    info!(error = ?dag_rpc_error, responder = responder, "fetch failure: target {} returned error", responder);
                },
                Err(err) => {
                    info!(error = ?err, responder = responder, "rpc failed to {}", responder);
                },
            }
        }
        Err(anyhow::anyhow!("Fetch with fallback failed"))
    }
}

pub struct BatchRequestHandler {
    payload_store: Arc<DagPayloadStore>,
}

impl BatchRequestHandler {
    pub fn new(payload_store: Arc<DagPayloadStore>) -> Self {
        Self { payload_store }
    }
}

#[async_trait]
impl RpcHandler for BatchRequestHandler {
    type Request = BatchRequest;
    type Response = BatchResponse;

    async fn process(&self, request: Self::Request) -> anyhow::Result<Self::Response> {
        let payload = self
            .payload_store
            .get(request.id(), request.batch_digest())?;
        Ok(BatchResponse::new(payload.as_ref().clone()))
    }
}
