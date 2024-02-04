// Copyright © Aptos Foundation

use crate::{
    certified_update_producer::CertifiedUpdateProducer,
    jwk_observer::JWKObserver,
    network::IncomingRpcRequest,
    types::{JWKConsensusMsg, ObservedUpdate, ObservedUpdateResponse},
};
use anyhow::{anyhow, bail, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::{bls12381::PrivateKey, SigningKey};
use aptos_logger::{debug, error, info};
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    jwks::{
        jwk::JWKMoveStruct, AllProvidersJWKs, Issuer, ObservedJWKs, ObservedJWKsUpdated,
        ProviderJWKs, QuorumCertifiedUpdate, SupportedOIDCProviders,
    },
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use futures_channel::oneshot;
use futures_util::{
    future::{join_all, AbortHandle},
    FutureExt, StreamExt,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

/// `JWKManager` executes per-issuer JWK consensus sessions
/// and updates validator txn pool with quorum-certified JWK updates.
pub struct JWKManager {
    /// Some useful metadata.
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,

    /// Used to sign JWK observations before sharing them with peers.
    consensus_key: Arc<PrivateKey>,

    /// The sub-process that collects JWK updates from peers and aggregate them into a quorum-certified JWK update.
    certified_update_producer: Arc<dyn CertifiedUpdateProducer>,

    /// When a quorum-certified JWK update is available, use this to put it into the validator transaction pool.
    vtxn_pool: VTxnPoolState,

    /// The JWK consensus states of all the issuers.
    states_by_issuer: HashMap<Issuer, PerProviderState>,

    /// Whether a CLOSE command has been received.
    stopped: bool,

    qc_update_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>,
    jwk_observers: Vec<JWKObserver>,
}

impl JWKManager {
    pub fn new(
        consensus_key: Arc<PrivateKey>,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        certified_update_producer: Arc<dyn CertifiedUpdateProducer>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        Self {
            consensus_key,
            my_addr,
            epoch_state,
            certified_update_producer,
            vtxn_pool,
            states_by_issuer: HashMap::default(),
            stopped: false,
            qc_update_tx: None,
            jwk_observers: vec![],
        }
    }

    pub async fn run(
        mut self,
        oidc_providers: Option<SupportedOIDCProviders>,
        observed_jwks: Option<ObservedJWKs>,
        mut jwk_updated_rx: aptos_channel::Receiver<(), ObservedJWKsUpdated>,
        mut rpc_req_rx: aptos_channel::Receiver<(), (AccountAddress, IncomingRpcRequest)>,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        self.reset_with_on_chain_state(observed_jwks.unwrap_or_default().into_providers_jwks())
            .unwrap();
        let (qc_update_tx, mut qc_update_rx) = aptos_channel::new(QueueStyle::FIFO, 100, None);
        self.qc_update_tx = Some(qc_update_tx);

        let (local_observation_tx, mut local_observation_rx) =
            aptos_channel::new(QueueStyle::KLAST, 100, None);

        self.jwk_observers = oidc_providers
            .unwrap_or_default()
            .into_provider_vec()
            .into_iter()
            .map(|provider| {
                JWKObserver::spawn(
                    self.my_addr,
                    provider.name.clone(),
                    provider.config_url.clone(),
                    Duration::from_secs(10),
                    local_observation_tx.clone(),
                )
            })
            .collect();

        let mut close_rx = close_rx.into_stream();

        while !self.stopped {
            let handle_result = tokio::select! {
                jwk_updated = jwk_updated_rx.select_next_some() => {
                    let ObservedJWKsUpdated { jwks, .. } = jwk_updated;
                    self.reset_with_on_chain_state(jwks)
                },
                (_sender, msg) = rpc_req_rx.select_next_some() => {
                    self.process_peer_request(msg)
                },
                qc_update = qc_update_rx.select_next_some() => {
                    self.process_quorum_certified_update(qc_update)
                },
                (issuer, jwks) = local_observation_rx.select_next_some() => {
                    let jwks = jwks.into_iter().map(JWKMoveStruct::from).collect();
                    self.process_new_observation(issuer, jwks)
                },
                ack_tx = close_rx.select_next_some() => {
                    self.tear_down(ack_tx.ok()).await
                }
            };

            if let Err(e) = handle_result {
                error!("[JWK] handling_err={}", e);
            }
        }
    }

    async fn tear_down(&mut self, ack_tx: Option<oneshot::Sender<()>>) -> Result<()> {
        self.stopped = true;
        let futures = std::mem::take(&mut self.jwk_observers)
            .into_iter()
            .map(JWKObserver::shutdown)
            .collect::<Vec<_>>();
        join_all(futures).await;
        if let Some(tx) = ack_tx {
            let _ = tx.send(());
        }
        Ok(())
    }

    /// Triggered by an observation thread periodically.
    pub fn process_new_observation(
        &mut self,
        issuer: Issuer,
        jwks: Vec<JWKMoveStruct>,
    ) -> Result<()> {
        let state = self.states_by_issuer.entry(issuer.clone()).or_default();
        state.observed = Some(jwks.clone());
        if state.observed.as_ref() != state.on_chain.as_ref().map(ProviderJWKs::jwks) {
            let observed = ProviderJWKs {
                issuer: issuer.clone(),
                version: state.on_chain_version() + 1,
                jwks,
            };
            let signature = self
                .consensus_key
                .sign(&observed)
                .map_err(|e| anyhow!("crypto material error occurred duing signing: {}", e))?;
            let abort_handle = self.certified_update_producer.start_produce(
                self.epoch_state.clone(),
                observed.clone(),
                self.qc_update_tx.clone(),
            );
            state.consensus_state = ConsensusState::InProgress {
                my_proposal: ObservedUpdate {
                    author: self.my_addr,
                    observed: observed.clone(),
                    signature,
                },
                abort_handle_wrapper: QuorumCertProcessGuard::new(abort_handle),
            };
            info!("[JWK] update observed, update={:?}", observed);
        }

        Ok(())
    }

    /// Invoked on start, or on on-chain JWK updated event.
    pub fn reset_with_on_chain_state(&mut self, on_chain_state: AllProvidersJWKs) -> Result<()> {
        debug!(
            "[JWK] reset_with_on_chain_state: BEGIN: on_chain_state={:?}",
            on_chain_state
        );
        let onchain_issuer_set: HashSet<Issuer> = on_chain_state
            .entries
            .iter()
            .map(|entry| entry.issuer.clone())
            .collect();
        self.states_by_issuer
            .retain(|issuer, _| onchain_issuer_set.contains(issuer));
        for on_chain_provider_jwks in on_chain_state.entries {
            let locally_cached = self
                .states_by_issuer
                .get(&on_chain_provider_jwks.issuer)
                .and_then(|s| s.on_chain.as_ref());
            if locally_cached == Some(&on_chain_provider_jwks) {
                // The on-chain update did not touch this provider.
                // The corresponding local state does not have to be reset.
            } else {
                self.states_by_issuer.insert(
                    on_chain_provider_jwks.issuer.clone(),
                    PerProviderState::new(on_chain_provider_jwks),
                );
            }
        }
        Ok(())
    }

    pub fn process_peer_request(&mut self, rpc_req: IncomingRpcRequest) -> Result<()> {
        let IncomingRpcRequest {
            msg,
            mut response_sender,
            sender,
        } = rpc_req;
        debug!(
            "[JWK] process_peer_request: sender={}, is_self={}",
            sender,
            sender == self.my_addr
        );
        match msg {
            JWKConsensusMsg::ObservationRequest(request) => {
                let state = self.states_by_issuer.entry(request.issuer).or_default();
                let response: Result<JWKConsensusMsg> = match &state.consensus_state {
                    ConsensusState::NotStarted => Err(anyhow!("observed update unavailable")),
                    ConsensusState::InProgress { my_proposal, .. }
                    | ConsensusState::Finished { my_proposal, .. } => Ok(
                        JWKConsensusMsg::ObservationResponse(ObservedUpdateResponse {
                            epoch: self.epoch_state.epoch,
                            update: my_proposal.clone(),
                        }),
                    ),
                };
                response_sender.send(response);
                Ok(())
            },
            _ => {
                bail!("unexpected rpc: {}", msg.name());
            },
        }
    }

    /// Triggered once the `certified_update_producer` produced a quorum-certified update.
    pub fn process_quorum_certified_update(&mut self, update: QuorumCertifiedUpdate) -> Result<()> {
        let issuer = update.update.issuer.clone();
        let state = self.states_by_issuer.entry(issuer.clone()).or_default();
        match &state.consensus_state {
            ConsensusState::InProgress { my_proposal, .. } => {
                //TODO: counters
                let txn = ValidatorTransaction::ObservedJWKUpdate(update.clone());
                let vtxn_guard =
                    self.vtxn_pool
                        .put(Topic::JWK_CONSENSUS(issuer), Arc::new(txn), None);
                state.consensus_state = ConsensusState::Finished {
                    vtxn_guard,
                    my_proposal: my_proposal.clone(),
                    quorum_certified: update.clone(),
                };
                info!("[JWK] qc update obtained, update={:?}", update);
                Ok(())
            },
            _ => Err(anyhow!(
                "qc update not expected for issuer {:?} in state {}",
                update.update.issuer,
                state.consensus_state.name()
            )),
        }
    }
}

/// An instance of this resource is created when `JWKManager` starts the QC update building process for an issuer.
/// Then `JWKManager` needs to hold it. Once this resource is dropped, the corresponding QC update process will be cancelled.
#[derive(Clone, Debug)]
pub struct QuorumCertProcessGuard {
    handle: Option<AbortHandle>,
}

impl QuorumCertProcessGuard {
    pub fn new(handle: AbortHandle) -> Self {
        Self {
            handle: Some(handle),
        }
    }

    #[cfg(test)]
    pub fn dummy() -> Self {
        Self { handle: None }
    }
}

impl Drop for QuorumCertProcessGuard {
    fn drop(&mut self) {
        let QuorumCertProcessGuard { handle } = self;
        if let Some(handle) = handle {
            handle.abort();
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConsensusState {
    NotStarted,
    InProgress {
        my_proposal: ObservedUpdate,
        abort_handle_wrapper: QuorumCertProcessGuard,
    },
    Finished {
        vtxn_guard: TxnGuard,
        my_proposal: ObservedUpdate,
        quorum_certified: QuorumCertifiedUpdate,
    },
}

impl PartialEq for ConsensusState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ConsensusState::NotStarted, ConsensusState::NotStarted) => true,
            (
                ConsensusState::InProgress {
                    my_proposal: update_0,
                    ..
                },
                ConsensusState::InProgress {
                    my_proposal: update_1,
                    ..
                },
            ) if update_0 == update_1 => true,
            (
                ConsensusState::Finished {
                    my_proposal: update_0,
                    ..
                },
                ConsensusState::Finished {
                    my_proposal: update_1,
                    ..
                },
            ) if update_0 == update_1 => true,
            _ => false,
        }
    }
}

impl Eq for ConsensusState {}

impl ConsensusState {
    pub fn name(&self) -> &str {
        match self {
            ConsensusState::NotStarted => "NotStarted",
            ConsensusState::InProgress { .. } => "InProgress",
            ConsensusState::Finished { .. } => "Finished",
        }
    }

    #[cfg(test)]
    pub fn my_proposal_cloned(&self) -> ObservedUpdate {
        match self {
            ConsensusState::InProgress { my_proposal, .. }
            | ConsensusState::Finished { my_proposal, .. } => my_proposal.clone(),
            _ => panic!("my_proposal unavailable"),
        }
    }
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self::NotStarted
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PerProviderState {
    pub on_chain: Option<ProviderJWKs>,
    pub observed: Option<Vec<JWKMoveStruct>>,
    pub consensus_state: ConsensusState,
}

impl PerProviderState {
    pub fn new(provider_jwks: ProviderJWKs) -> Self {
        Self {
            on_chain: Some(provider_jwks),
            observed: None,
            consensus_state: ConsensusState::NotStarted,
        }
    }

    pub fn on_chain_version(&self) -> u64 {
        self.on_chain
            .as_ref()
            .map_or(0, |provider_jwks| provider_jwks.version)
    }
}

#[cfg(test)]
pub mod tests;
