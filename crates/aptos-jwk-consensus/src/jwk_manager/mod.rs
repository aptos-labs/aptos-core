// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwk_observer::JWKObserver,
    mode::per_issuer::PerIssuerMode,
    network::IncomingRpcRequest,
    types::{
        ConsensusState, JWKConsensusMsg, ObservedUpdate, ObservedUpdateResponse,
        QuorumCertProcessGuard,
    },
    update_certifier::TUpdateCertifier,
    TConsensusManager,
};
use anyhow::{anyhow, bail, Context, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::{bls12381::PrivateKey, SigningKey};
use aptos_logger::{debug, error, info, warn};
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    jwks::{
        jwk::JWKMoveStruct, AllProvidersJWKs, Issuer, OIDCProvider, ObservedJWKs,
        ObservedJWKsUpdated, ProviderJWKs, QuorumCertifiedUpdate, SupportedOIDCProviders,
    },
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::VTxnPoolState;
use futures_channel::oneshot;
use futures_util::{future::join_all, FutureExt, StreamExt};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

/// `JWKManager` executes per-issuer JWK consensus sessions
/// and updates validator txn pool with quorum-certified JWK updates.
pub struct IssuerLevelConsensusManager {
    /// Some useful metadata.
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,

    /// Used to sign JWK observations before sharing them with peers.
    consensus_key: Arc<PrivateKey>,

    /// The sub-process that collects JWK updates from peers and aggregate them into a quorum-certified JWK update.
    update_certifier: Arc<dyn TUpdateCertifier<PerIssuerMode>>,

    /// When a quorum-certified JWK update is available, use this to put it into the validator transaction pool.
    vtxn_pool: VTxnPoolState,

    /// The JWK consensus states of all the issuers.
    states_by_issuer: HashMap<Issuer, PerProviderState>,

    /// Whether a CLOSE command has been received.
    stopped: bool,

    qc_update_tx: aptos_channel::Sender<Issuer, QuorumCertifiedUpdate>,
    qc_update_rx: aptos_channel::Receiver<Issuer, QuorumCertifiedUpdate>,
    jwk_observers: Vec<JWKObserver>,
}

impl IssuerLevelConsensusManager {
    pub fn new(
        consensus_key: Arc<PrivateKey>,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        update_certifier: Arc<dyn TUpdateCertifier<PerIssuerMode>>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        let (qc_update_tx, qc_update_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        Self {
            consensus_key,
            my_addr,
            epoch_state,
            update_certifier,
            vtxn_pool,
            states_by_issuer: HashMap::default(),
            stopped: false,
            qc_update_tx,
            qc_update_rx,
            jwk_observers: vec![],
        }
    }
}

#[async_trait::async_trait]
impl TConsensusManager for IssuerLevelConsensusManager {
    async fn run(
        self: Box<Self>,
        oidc_providers: Option<SupportedOIDCProviders>,
        observed_jwks: Option<ObservedJWKs>,
        mut jwk_updated_rx: aptos_channel::Receiver<(), ObservedJWKsUpdated>,
        mut rpc_req_rx: aptos_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        let mut this = self;
        this.reset_with_on_chain_state(observed_jwks.unwrap_or_default().into_providers_jwks())
            .unwrap();

        let (local_observation_tx, mut local_observation_rx) =
            aptos_channel::new(QueueStyle::KLAST, 100, None);

        this.jwk_observers = oidc_providers
            .unwrap_or_default()
            .into_provider_vec()
            .into_iter()
            .filter_map(|provider| {
                let OIDCProvider { name, config_url } = provider;
                let maybe_issuer = String::from_utf8(name);
                let maybe_config_url = String::from_utf8(config_url);
                match (maybe_issuer, maybe_config_url) {
                    (Ok(issuer), Ok(config_url)) => Some(JWKObserver::spawn(
                        this.epoch_state.epoch,
                        this.my_addr,
                        issuer,
                        config_url,
                        Duration::from_secs(10),
                        local_observation_tx.clone(),
                    )),
                    (maybe_issuer, maybe_config_url) => {
                        warn!(
                            "unable to spawn observer, issuer={:?}, config_url={:?}",
                            maybe_issuer, maybe_config_url
                        );
                        None
                    },
                }
            })
            .collect();

        let mut close_rx = close_rx.into_stream();

        while !this.stopped {
            let handle_result = tokio::select! {
                jwk_updated = jwk_updated_rx.select_next_some() => {
                    let ObservedJWKsUpdated { jwks, .. } = jwk_updated;
                    this.reset_with_on_chain_state(jwks)
                },
                (_sender, msg) = rpc_req_rx.select_next_some() => {
                    this.process_peer_request(msg)
                },
                qc_update = this.qc_update_rx.select_next_some() => {
                    this.process_quorum_certified_update(qc_update)
                },
                (issuer, jwks) = local_observation_rx.select_next_some() => {
                    let jwks = jwks.into_iter().map(JWKMoveStruct::from).collect();
                    this.process_new_observation(issuer, jwks)
                },
                ack_tx = close_rx.select_next_some() => {
                    this.tear_down(ack_tx.ok()).await
                }
            };

            if let Err(e) = handle_result {
                error!(
                    epoch = this.epoch_state.epoch,
                    "JWKManager handling error: {}", e
                );
            }
        }
    }
}

impl IssuerLevelConsensusManager {
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
        debug!(
            epoch = self.epoch_state.epoch,
            issuer = String::from_utf8(issuer.clone()).ok(),
            "Processing new observation."
        );
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
                .context("process_new_observation failed with signing error")?;
            let abort_handle = self
                .update_certifier
                .start_produce(
                    self.epoch_state.clone(),
                    observed.clone(),
                    self.qc_update_tx.clone(),
                )
                .context(
                    "process_new_observation failed with update_certifier.start_produce failure",
                )?;
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
        info!(
            epoch = self.epoch_state.epoch,
            "reset_with_on_chain_state starting."
        );
        let onchain_issuer_set: HashSet<Issuer> = on_chain_state
            .entries
            .iter()
            .map(|entry| entry.issuer.clone())
            .collect();
        let local_issuer_set: HashSet<Issuer> = self.states_by_issuer.keys().cloned().collect();

        for issuer in local_issuer_set.difference(&onchain_issuer_set) {
            info!(
                epoch = self.epoch_state.epoch,
                op = "delete",
                issuer = issuer.clone(),
                "reset_with_on_chain_state"
            );
        }

        self.states_by_issuer
            .retain(|issuer, _| onchain_issuer_set.contains(issuer));
        for on_chain_provider_jwks in on_chain_state.entries {
            let issuer = on_chain_provider_jwks.issuer.clone();
            let locally_cached = self
                .states_by_issuer
                .get(&on_chain_provider_jwks.issuer)
                .and_then(|s| s.on_chain.as_ref());
            if locally_cached == Some(&on_chain_provider_jwks) {
                // The on-chain update did not touch this provider.
                // The corresponding local state does not have to be reset.
                info!(
                    epoch = self.epoch_state.epoch,
                    op = "no-op",
                    issuer = issuer,
                    "reset_with_on_chain_state"
                );
            } else {
                let old_value = self.states_by_issuer.insert(
                    on_chain_provider_jwks.issuer.clone(),
                    PerProviderState::new(on_chain_provider_jwks),
                );
                let op = if old_value.is_some() {
                    "update"
                } else {
                    "insert"
                };
                info!(
                    epoch = self.epoch_state.epoch,
                    op = op,
                    issuer = issuer,
                    "reset_with_on_chain_state"
                );
            }
        }
        info!(
            epoch = self.epoch_state.epoch,
            "reset_with_on_chain_state finished."
        );
        Ok(())
    }

    pub fn process_peer_request(&mut self, rpc_req: IncomingRpcRequest) -> Result<()> {
        let IncomingRpcRequest {
            msg,
            mut response_sender,
            ..
        } = rpc_req;
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

    /// Triggered once the `update_certifier` produced a quorum-certified update.
    pub fn process_quorum_certified_update(&mut self, update: QuorumCertifiedUpdate) -> Result<()> {
        let issuer = update.update.issuer.clone();
        info!(
            epoch = self.epoch_state.epoch,
            issuer = String::from_utf8(issuer.clone()).ok(),
            version = update.update.version,
            "JWKManager processing certified update."
        );
        let state = self.states_by_issuer.entry(issuer.clone()).or_default();
        match &state.consensus_state {
            ConsensusState::InProgress { my_proposal, .. } => {
                //TODO: counters
                let txn = ValidatorTransaction::ObservedJWKUpdate(update.clone());
                let vtxn_guard =
                    self.vtxn_pool
                        .put(Topic::JWK_CONSENSUS(issuer.clone()), Arc::new(txn), None);
                state.consensus_state = ConsensusState::Finished {
                    vtxn_guard,
                    my_proposal: my_proposal.clone(),
                    quorum_certified: update.clone(),
                };
                info!(
                    epoch = self.epoch_state.epoch,
                    issuer = String::from_utf8(issuer).ok(),
                    version = update.update.version,
                    "certified update accepted."
                );
                Ok(())
            },
            _ => Err(anyhow!(
                "qc update not expected for issuer {:?} in state {}",
                String::from_utf8(issuer.clone()),
                state.consensus_state.name()
            )),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PerProviderState {
    pub on_chain: Option<ProviderJWKs>,
    pub observed: Option<Vec<JWKMoveStruct>>,
    pub consensus_state: ConsensusState<ObservedUpdate>,
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
