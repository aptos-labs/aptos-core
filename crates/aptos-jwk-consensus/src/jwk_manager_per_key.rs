// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwk_observer::JWKObserver,
    mode::per_key::PerKeyMode,
    network::IncomingRpcRequest,
    types::{
        ConsensusState, JWKConsensusMsg, ObservedKeyLevelUpdate, ObservedKeyLevelUpdateRequest,
        ObservedUpdate, ObservedUpdateResponse, QuorumCertProcessGuard,
    },
    update_certifier::{TUpdateCertifier, UpdateCertifier},
    TConsensusManager,
};
use anyhow::{anyhow, bail, Context, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::{bls12381::PrivateKey, SigningKey};
use aptos_logger::{debug, error, info, warn};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    jwks::{
        jwk::JWK, AllProvidersJWKs, Issuer, KeyLevelUpdate, OIDCProvider, ObservedJWKs,
        ObservedJWKsUpdated, ProviderJWKsIndexed, QuorumCertifiedUpdate, SupportedOIDCProviders,
        KID,
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
use tokio_retry::strategy::ExponentialBackoff;

/// `JWKManager` executes per-issuer JWK consensus sessions
/// and updates validator txn pool with quorum-certified JWK updates.
pub struct KeyLevelConsensusManager {
    /// Some useful metadata.
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,

    /// Used to sign JWK observations before sharing them with peers.
    consensus_key: Arc<PrivateKey>,

    /// The sub-process that collects JWK updates from peers and aggregate them into a quorum-certified JWK update.
    update_certifier: Arc<dyn TUpdateCertifier<PerKeyMode>>,

    /// When a quorum-certified JWK update is available, use this to put it into the validator transaction pool.
    vtxn_pool: VTxnPoolState,

    onchain_jwks: HashMap<Issuer, ProviderJWKsIndexed>,
    /// The JWK consensus states of all (issuer,kid)s.
    states_by_key: HashMap<(Issuer, KID), ConsensusState<ObservedKeyLevelUpdate>>,

    /// Whether a CLOSE command has been received.
    stopped: bool,

    qc_update_tx: aptos_channel::Sender<(Issuer, KID), QuorumCertifiedUpdate>,
    qc_update_rx: aptos_channel::Receiver<(Issuer, KID), QuorumCertifiedUpdate>,
    jwk_observers: Vec<JWKObserver>,
}

impl KeyLevelConsensusManager {
    pub fn new(
        consensus_key: Arc<PrivateKey>,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        rb: ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        let update_certifier = Arc::new(UpdateCertifier::new(rb));

        let (qc_update_tx, qc_update_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        Self {
            consensus_key,
            my_addr,
            epoch_state,
            update_certifier,
            vtxn_pool,
            onchain_jwks: HashMap::default(),
            states_by_key: HashMap::default(),
            stopped: false,
            qc_update_tx,
            qc_update_rx,
            jwk_observers: vec![],
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
    pub fn process_new_observation(&mut self, issuer: Issuer, jwks: Vec<JWK>) -> Result<()> {
        debug!(
            epoch = self.epoch_state.epoch,
            issuer = String::from_utf8(issuer.clone()).ok(),
            "Processing new observation."
        );
        let observed_jwks_by_kid: HashMap<KID, JWK> =
            jwks.into_iter().map(|jwk| (jwk.id(), jwk)).collect();
        let effectively_onchain = self
            .onchain_jwks
            .get(&issuer)
            .cloned()
            .unwrap_or_else(|| ProviderJWKsIndexed::new(issuer.clone()));
        let all_kids: HashSet<KID> = effectively_onchain
            .jwks
            .keys()
            .chain(observed_jwks_by_kid.keys())
            .cloned()
            .collect();
        for kid in all_kids {
            let onchain = effectively_onchain.jwks.get(&kid);
            let observed = observed_jwks_by_kid.get(&kid);
            match (onchain, observed) {
                (Some(x), Some(y)) => {
                    if x == y {
                        // No change, drop any in-progress consensus.
                        self.states_by_key.remove(&(issuer.clone(), kid.clone()));
                    } else {
                        // Update detected.
                        let update = KeyLevelUpdate {
                            issuer: issuer.clone(),
                            base_version: effectively_onchain.version,
                            kid: kid.clone(),
                            to_upsert: Some(y.clone()),
                        };
                        self.maybe_start_consensus(update)
                            .context("process_new_observation failed at upsert consensus init")?;
                    }
                },
                (None, Some(y)) => {
                    // Insert detected.
                    let update = KeyLevelUpdate {
                        issuer: issuer.clone(),
                        base_version: effectively_onchain.version,
                        kid: kid.clone(),
                        to_upsert: Some(y.clone()),
                    };
                    self.maybe_start_consensus(update)
                        .context("process_new_observation failed at upsert consensus init")?;
                },
                (Some(_), None) => {
                    // Delete detected.
                    let update = KeyLevelUpdate {
                        issuer: issuer.clone(),
                        base_version: effectively_onchain.version,
                        kid: kid.clone(),
                        to_upsert: None,
                    };
                    self.maybe_start_consensus(update)
                        .context("process_new_observation failed at deletion consensus init")?;
                },
                (None, None) => {
                    unreachable!()
                },
            }
        }

        Ok(())
    }

    fn maybe_start_consensus(&mut self, update: KeyLevelUpdate) -> Result<()> {
        let consensus_already_started = match self
            .states_by_key
            .get(&(update.issuer.clone(), update.kid.clone()))
            .cloned()
        {
            Some(ConsensusState::InProgress { my_proposal, .. })
            | Some(ConsensusState::Finished { my_proposal, .. }) => {
                my_proposal.observed.to_upsert == update.to_upsert
            },
            _ => false,
        };

        if consensus_already_started {
            return Ok(());
        }

        let issuer_level_repr = update
            .as_issuer_level_repr()
            .context("initiate_key_level_consensus failed at repr conversion")?;
        let signature = self
            .consensus_key
            .sign(&issuer_level_repr)
            .context("crypto material error occurred during signing")?;

        let update_translated = update
            .as_issuer_level_repr()
            .context("maybe_start_consensus failed at update translation")?;
        let abort_handle = self
            .update_certifier
            .start_produce(
                self.epoch_state.clone(),
                update_translated,
                self.qc_update_tx.clone(),
            )
            .context("maybe_start_consensus failed at update_certifier.start_produce")?;

        self.states_by_key.insert(
            (update.issuer.clone(), update.kid.clone()),
            ConsensusState::InProgress {
                my_proposal: ObservedKeyLevelUpdate {
                    author: self.my_addr,
                    observed: update,
                    signature,
                },
                abort_handle_wrapper: QuorumCertProcessGuard {
                    handle: abort_handle,
                },
            },
        );

        Ok(())
    }

    /// Invoked on start, or on on-chain JWK updated event.
    pub fn reset_with_on_chain_state(&mut self, on_chain_state: AllProvidersJWKs) -> Result<()> {
        info!(
            epoch = self.epoch_state.epoch,
            "reset_with_on_chain_state starting."
        );

        let new_onchain_jwks = on_chain_state.indexed().context(
            "KeyLevelJWKManager::reset_with_on_chain_state failed at onchain state indexing",
        )?;
        // for an existing state entry (iss, kid) -> state, discard it unless `new_onchain_jwks[iss].version == self.onchain_jwks[iss].version`.
        self.states_by_key.retain(|(issuer, _), _| {
            new_onchain_jwks
                .get(issuer)
                .map(|jwks| jwks.version)
                .unwrap_or_default()
                == self
                    .onchain_jwks
                    .get(issuer)
                    .map(|jwks| jwks.version)
                    .unwrap_or_default()
        });

        self.onchain_jwks = new_onchain_jwks;

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
            JWKConsensusMsg::KeyLevelObservationRequest(request) => {
                let ObservedKeyLevelUpdateRequest { issuer, kid, .. } = request;
                let consensus_state = self
                    .states_by_key
                    .entry((issuer.clone(), kid.clone()))
                    .or_default();
                let response: Result<JWKConsensusMsg> = match &consensus_state {
                    ConsensusState::NotStarted => {
                        debug!(
                            issuer = String::from_utf8(issuer.clone()).ok(),
                            kid = String::from_utf8(kid.clone()).ok(),
                            "key-level jwk consensus not started"
                        );
                        return Ok(());
                    },
                    ConsensusState::InProgress { my_proposal, .. }
                    | ConsensusState::Finished { my_proposal, .. } => Ok(
                        JWKConsensusMsg::ObservationResponse(ObservedUpdateResponse {
                            epoch: self.epoch_state.epoch,
                            update: ObservedUpdate {
                                author: self.my_addr,
                                observed: my_proposal
                                    .observed
                                    .as_issuer_level_repr()
                                    .context("process_peer_request failed with repr conversion")?,
                                signature: my_proposal.signature.clone(),
                            },
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

    /// Triggered once the `update_certifier` produced a certified key-level update.
    pub fn process_quorum_certified_update(
        &mut self,
        issuer_level_repr: QuorumCertifiedUpdate,
    ) -> Result<()> {
        let key_level_update =
            KeyLevelUpdate::try_from_issuer_level_repr(&issuer_level_repr.update)
                .context("process_quorum_certified_update failed with repr err")?;
        let issuer = &key_level_update.issuer;
        let issuer_str = String::from_utf8(issuer.clone()).ok();
        let kid = &key_level_update.kid;
        let kid_str = String::from_utf8(kid.clone()).ok();
        info!(
            epoch = self.epoch_state.epoch,
            issuer = issuer_str,
            kid = kid_str,
            base_version = key_level_update.base_version,
            "KeyLevelJWKManager processing certified key-level update."
        );
        let state = self
            .states_by_key
            .entry((issuer.clone(), kid.clone()))
            .or_default();
        match state {
            ConsensusState::InProgress { my_proposal, .. } => {
                let topic = Topic::JWK_CONSENSUS_PER_KEY_MODE {
                    issuer: issuer.clone(),
                    kid: kid.clone(),
                };
                let txn = ValidatorTransaction::ObservedJWKUpdate(issuer_level_repr.clone());
                let vtxn_guard = self.vtxn_pool.put(topic, Arc::new(txn), None);
                *state = ConsensusState::Finished {
                    vtxn_guard,
                    my_proposal: my_proposal.clone(),
                    quorum_certified: issuer_level_repr,
                };
                info!(
                    epoch = self.epoch_state.epoch,
                    issuer = issuer_str,
                    kid = kid_str,
                    base_version = key_level_update.base_version,
                    "certified key-level update accepted."
                );
                Ok(())
            },
            _ => Err(anyhow!(
                "qc update not expected for issuer {:?} in state {}",
                String::from_utf8(issuer.clone()),
                state.name()
            )),
        }
    }
}

#[async_trait::async_trait]
impl TConsensusManager for KeyLevelConsensusManager {
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
                    this.process_new_observation(issuer, jwks)
                },
                ack_tx = close_rx.select_next_some() => {
                    this.tear_down(ack_tx.ok()).await
                }
            };

            if let Err(e) = handle_result {
                error!(
                    epoch = this.epoch_state.epoch,
                    "KeyLevelJWKManager error from handling: {}", e
                );
            }
        }
    }
}
