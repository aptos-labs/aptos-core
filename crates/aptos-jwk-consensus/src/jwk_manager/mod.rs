// Copyright © Aptos Foundation

use crate::{
    network::IncomingRpcRequest,
    types::{JWKConsensusMsg, ObservedUpdate, ObservedUpdateResponse},
};
use anyhow::{anyhow, bail, Result};
use aptos_channels::aptos_channel;
use aptos_crypto::{bls12381::PrivateKey, SigningKey};
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    jwks::{jwk::JWKMoveStruct, Issuer, ObservedJWKs, ProviderJWKs, QuorumCertifiedUpdate},
    validator_txn::ValidatorTransaction,
};
use aptos_validator_transaction_pool as vtxn_pool;
use certified_update_producer::CertifiedUpdateProducer;
use futures_util::future::AbortHandle;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

pub mod certified_update_producer;

/// `JWKManager` executes per-issuer JWK consensus sessions
/// and updates validator txn pool with quorum-certified JWK updates.
pub struct JWKManager {
    signing_key: PrivateKey,
    my_addr: AccountAddress,
    epoch_state: EpochState,
    certified_update_producer: Arc<dyn CertifiedUpdateProducer>,
    certified_update_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    states_by_issuer: HashMap<Issuer, PerProviderState>,
    _stopped: bool,
}

impl JWKManager {
    pub fn new(
        signing_key: PrivateKey,
        my_addr: AccountAddress,
        epoch_state: EpochState,
        certified_update_producer: Arc<dyn CertifiedUpdateProducer>,
        vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    ) -> Self {
        Self {
            signing_key,
            my_addr,
            epoch_state,
            certified_update_producer,
            certified_update_tx: None,
            vtxn_pool_write_cli,
            states_by_issuer: HashMap::default(),
            _stopped: false,
        }
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
                jwks: jwks.clone(),
            };
            let signature = self
                .signing_key
                .sign(&observed)
                .map_err(|e| anyhow!("crypto material error occurred duing signing: {}", e))?;
            let abort_handle = self.certified_update_producer.start_produce(
                self.epoch_state.clone(),
                observed.clone(),
                self.certified_update_tx.clone(),
            );
            state.consensus_state = ConsensusState::InProgress {
                my_proposal: ObservedUpdate {
                    author: self.my_addr,
                    observed,
                    signature,
                },
                abort_handle_wrapper: AbortHandleWrapper::new(abort_handle),
            };
        }

        Ok(())
    }

    /// Invoked on start, or on on-chain JWK updated event.
    pub fn reset_with_on_chain_state(&mut self, on_chain_state: ObservedJWKs) -> Result<()> {
        self.states_by_issuer = on_chain_state
            .jwks
            .entries
            .iter()
            .map(|provider_jwks| {
                (
                    provider_jwks.issuer.clone(),
                    PerProviderState::new(provider_jwks.clone()),
                )
            })
            .collect();
        self.vtxn_pool_write_cli.put(None);
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
        let state = self
            .states_by_issuer
            .entry(update.observed.issuer.clone())
            .or_default();
        match &state.consensus_state {
            ConsensusState::InProgress { my_proposal, .. } => {
                //TODO: counters
                state.consensus_state = ConsensusState::Finished {
                    my_proposal: my_proposal.clone(),
                    quorum_certified: update,
                };
                self.update_vtxn_pool()?;
                Ok(())
            },
            _ => Err(anyhow!(
                "qc update not expected for issuer {:?} in state {}",
                update.observed.issuer,
                state.consensus_state.name()
            )),
        }
    }

    fn update_vtxn_pool(&mut self) -> Result<()> {
        let updates: BTreeMap<Issuer, QuorumCertifiedUpdate> = self
            .states_by_issuer
            .iter()
            .filter_map(
                |(issuer, per_provider_state)| match &per_provider_state.consensus_state {
                    ConsensusState::Finished {
                        quorum_certified, ..
                    } => Some((issuer.clone(), quorum_certified.clone())),
                    _ => None,
                },
            )
            .collect();
        let txn = ValidatorTransaction::ObservedJWKsUpdates { updates };
        self.vtxn_pool_write_cli.put(Some(Arc::new(txn)));
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AbortHandleWrapper {
    handle: Option<AbortHandle>,
}

impl AbortHandleWrapper {
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

impl Drop for AbortHandleWrapper {
    fn drop(&mut self) {
        let AbortHandleWrapper { handle } = self;
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
        abort_handle_wrapper: AbortHandleWrapper,
    },
    Finished {
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
