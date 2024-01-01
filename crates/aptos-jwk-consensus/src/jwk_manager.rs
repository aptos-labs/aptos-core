// Copyright © Aptos Foundation

use anyhow::{anyhow, bail, Result};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use futures_util::future::AbortHandle;
use serde::{Deserialize, Serialize};
use aptos_channels::aptos_channel;
use aptos_crypto::{bls12381, SigningKey, Uniform};
use aptos_crypto::bls12381::{PrivateKey, PublicKey, Signature};
use aptos_infallible::{Mutex, RwLock};
use aptos_types::account_address::AccountAddress;
use aptos_types::epoch_state::EpochState;
use aptos_types::jwks::{Issuer, issuer_from_str, JWK, JWKs, ObservedJWKs, ProviderJWKs, QuorumCertifiedUpdate};
use aptos_types::jwks::unsupported::UnsupportedJWK;
use aptos_types::validator_config::ValidatorConfig;
use aptos_types::validator_txn::{Topic, ValidatorTransaction};
use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier};
use crate::JWKConsensusMsg;
use crate::network::{DummyRpcResponseSender, IncomingRpcRequest};
use aptos_validator_transaction_pool as vtxn_pool;
use aptos_validator_transaction_pool::TransactionFilter;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct ObservedUpdate {
    author: AccountAddress,
    payload: ProviderJWKs,
    signature: bls12381::Signature,
}

#[derive(Clone, Debug)]
struct AbortHandleWrapper {
    handle: Option<AbortHandle>,
}

impl AbortHandleWrapper {
    pub fn new(handle: AbortHandle) -> Self {
        Self {
            handle: Some(handle)
        }
    }

    #[cfg(test)]
    pub fn dummy() -> Self {
        Self {
            handle: None,
        }
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
            (ConsensusState::InProgress { my_proposal: update_0, .. }, ConsensusState::InProgress { my_proposal: update_1, .. }) if update_0 == update_1 => true,
            (ConsensusState::Finished { my_proposal: update_0, .. }, ConsensusState::Finished { my_proposal: update_1, .. }) if update_0 == update_1 => true,
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
    pub observed: Option<Vec<JWK>>,
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
        self.on_chain.as_ref().map_or(0, |provider_jwks| provider_jwks.version)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservedUpdateRequest {
    pub issuer: Issuer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservedUpdateResponse {
    update: ObservedUpdate,
}

/// A sub-process of the whole DKG process.
/// Once invoked by `DKGManager` to `start_produce`,
/// it starts producing an `AggDKGNode` and returns an abort handle.
/// Once an `AggDKGNode` is available, it is sent back via a channel given earlier.
pub trait CertifiedUpdateProducer: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: EpochState,
        payload: ProviderJWKs,
        agg_node_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>,
    ) -> AbortHandle;
}

pub struct DummyCertifiedUpdateProducer {
    pub invocations: Mutex<Vec<(EpochState, ProviderJWKs)>>,
}

impl DummyCertifiedUpdateProducer {
    pub fn new() -> Self {
        Self {
            invocations: Mutex::new(vec![]),
        }
    }
}

impl CertifiedUpdateProducer for DummyCertifiedUpdateProducer {
    fn start_produce(&self, epoch_state: EpochState, payload: ProviderJWKs, _agg_node_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>) -> AbortHandle {
        self.invocations.lock().push((epoch_state, payload));
        let (abort_handle, _) = AbortHandle::new_pair();
        abort_handle

    }
}

pub struct JWKManager {
    signing_key: bls12381::PrivateKey,
    my_addr: AccountAddress,
    epoch_state: EpochState,
    certified_update_producer: Arc<dyn CertifiedUpdateProducer>,
    certified_update_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    states_by_issuer: HashMap<Issuer, PerProviderState>,
    stopped: bool,
}

impl JWKManager {
    pub fn new(
        signing_key: bls12381::PrivateKey,
        my_addr: AccountAddress,
        epoch_state: EpochState,
        certified_update_producer: Arc<dyn CertifiedUpdateProducer>,
        vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>
    ) -> Self {
        Self {
            signing_key,
            my_addr,
            epoch_state,
            certified_update_producer,
            certified_update_tx: None,
            vtxn_pool_write_cli,
            states_by_issuer: HashMap::default(),
            stopped: false,
        }
    }

    fn process_new_observation(&mut self, issuer: Issuer, jwks: Vec<JWK>) -> Result<()> {
        let state = self.states_by_issuer.entry(issuer.clone()).or_insert_with(PerProviderState::default);
        state.observed = Some(jwks.clone());
        if state.observed.as_ref() != state.on_chain.as_ref().map(ProviderJWKs::jwks) {
            let payload = ProviderJWKs {
                issuer: issuer.clone(),
                version: state.on_chain_version() + 1,
                jwks: jwks.clone(),
            };
            let signature = self.signing_key.sign(&payload).map_err(|e| anyhow!("crypto material error occurred duing signing: {}", e))?;
            let abort_handle = self.certified_update_producer.start_produce(self.epoch_state.clone(), payload.clone(), self.certified_update_tx.clone());
            state.consensus_state = ConsensusState::InProgress {
                my_proposal: ObservedUpdate {
                    author: self.my_addr,
                    payload,
                    signature,
                },
                abort_handle_wrapper: AbortHandleWrapper::new(abort_handle),
            };
        }

        Ok(())
    }

    fn reset_with_on_chain_state(&mut self, on_chain_state: ObservedJWKs) -> Result<()> {
        self.states_by_issuer = on_chain_state.jwks.entries.iter()
            .map(|provider_jwks| (provider_jwks.issuer.clone(), PerProviderState::new(provider_jwks.clone())))
            .collect();
        self.vtxn_pool_write_cli.put(None);
        Ok(())
    }


    fn process_peer_request(&mut self, rpc_req: IncomingRpcRequest) -> Result<()> {
        let IncomingRpcRequest { msg, sender, mut response_sender } = rpc_req;
        match msg {
            JWKConsensusMsg::ObservationRequest(request) => {
                let state = self.states_by_issuer.entry(request.issuer).or_insert_with(PerProviderState::default);
                let response: Result<JWKConsensusMsg> = match &state.consensus_state {
                    ConsensusState::NotStarted => {
                        Err(anyhow!("observed update unavailable"))
                    },
                    ConsensusState::InProgress { my_proposal, .. }
                    | ConsensusState::Finished { my_proposal, .. } => {
                        Ok(JWKConsensusMsg::ObservationResponse(
                            ObservedUpdateResponse {
                                update: my_proposal.clone()
                            }
                        ))
                    }
                };
                response_sender.send(response);
                Ok(())
            },
            _ => {
                bail!("unexpected rpc: {}", msg.name());
            },
        }
    }

    fn process_quorum_certified_update(&mut self, update: QuorumCertifiedUpdate) -> Result<()> {
        let state = self.states_by_issuer.entry(update.payload.issuer.clone()).or_insert_with(PerProviderState::default);
        match &state.consensus_state {
            ConsensusState::InProgress { my_proposal, .. } => {
                //TODO: counters
                state.consensus_state = ConsensusState::Finished { my_proposal: my_proposal.clone(), quorum_certified: update };
                self.update_vtxn_pool()?;
                Ok(())
            },
            _ => Err(anyhow!("qc update not expected for issuer {:?} in state {}", update.payload.issuer, state.consensus_state.name())),
        }
    }

    fn update_vtxn_pool(&mut self) -> Result<()> {
        let updates: BTreeMap<Issuer, QuorumCertifiedUpdate> = self.states_by_issuer.iter()
            .filter_map(|(issuer, per_provider_state)| {
                match &per_provider_state.consensus_state {
                    ConsensusState::Finished { quorum_certified, .. } => Some((issuer.clone(), quorum_certified.clone())),
                    _ => None,
                }
            })
            .collect();
        let txn = ValidatorTransaction::ObservedJWKsUpdates { updates };
        self.vtxn_pool_write_cli.put(Some(Arc::new(txn)));
        Ok(())
    }
}

#[tokio::test]
async fn test_jwk_manager_state_transition() {
    let private_keys: Vec<PrivateKey> =
        (0..4).map(|_| PrivateKey::generate_for_testing()).collect();
    let public_keys: Vec<PublicKey> = private_keys.iter().map(PublicKey::from).collect();
    let addrs: Vec<AccountAddress> = (0..4).map(|_| AccountAddress::random()).collect();
    let voting_powers: Vec<u64> = vec![1, 1, 1, 1];
    let validator_consensus_infos: Vec<ValidatorConsensusInfo> = (0..4)
        .map(|i| ValidatorConsensusInfo::new(addrs[i], public_keys[i].clone(), voting_powers[i]))
        .collect();
    let validator_configs: Vec<ValidatorConfig> = (0..4)
        .map(|i| ValidatorConfig::new(public_keys[i].clone(), vec![], vec![], i as u64))
        .collect();
    let epoch_state = EpochState {
        epoch: 999,
        verifier: ValidatorVerifier::new(validator_consensus_infos.clone()),
    };

    let certified_update_producer = DummyCertifiedUpdateProducer::new();
    let (vtxn_pool_read_cli, mut vtxn_pool_write_clis) = aptos_validator_transaction_pool::new(vec![(Topic::JWK_CONSENSUS, None)]);
    let vtxn_pool_write_cli = vtxn_pool_write_clis.pop().unwrap();
    let mut jwk_manager = JWKManager::new(private_keys[0].clone(), addrs[0].clone(), epoch_state, Arc::new(certified_update_producer), Arc::new(vtxn_pool_write_cli));

    // Introducing issuers. Alice and Bob are existing issuers. Carl was added in the last epoch so no JWKs of Carl is on chain.
    let issuer_alice = issuer_from_str("https://alice.info");
    let issuer_bob = issuer_from_str("https://bob.io");
    let issuer_carl = issuer_from_str("https://carl.dev");
    let alice_jwks = vec![
        JWK::new_unsupported(UnsupportedJWK::new_for_test("alice_jwk_id_0", "jwk_payload_0")),
        JWK::new_unsupported(UnsupportedJWK::new_for_test("alice_jwk_id_1", "jwk_payload_1")),
    ];
    let bob_jwks = vec![
        JWK::new_unsupported(UnsupportedJWK::new_for_test("bob_jwk_id_0", "jwk_payload_2")),
        JWK::new_unsupported(UnsupportedJWK::new_for_test("bob_jwk_id_1", "jwk_payload_3")),
    ];
    let on_chain_state_alice_v111 = ProviderJWKs {
        issuer: issuer_alice.clone(),
        version: 111,
        jwks: alice_jwks.clone(),
    };

    let on_chain_state_bob_v222 = ProviderJWKs {
        issuer: issuer_bob.clone(),
        version: 222,
        jwks: bob_jwks.clone(),
    };

    // Initially, only Alice and Bob have JWKs on chain.
    let initial_on_chain_state = ObservedJWKs {
        jwks: JWKs { entries: vec![on_chain_state_alice_v111.clone(), on_chain_state_bob_v222.clone()] },
    };

    // On start, JWKManager is always initialized with the on-chain state.
    let handle_result = jwk_manager.reset_with_on_chain_state(initial_on_chain_state);
    assert!(handle_result.is_ok());

    let mut expected_states = HashMap::from([
        (issuer_alice.clone(), PerProviderState {
            on_chain: Some(on_chain_state_alice_v111.clone()),
            observed: None,
            consensus_state: ConsensusState::NotStarted,
        }),
        (issuer_bob.clone(), PerProviderState {
            on_chain: Some(on_chain_state_bob_v222.clone()),
            observed: None,
            consensus_state: ConsensusState::NotStarted,
        }),
    ]);
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    let rpc_response_collector = Arc::new(RwLock::new(vec![]));

    // For issuer Bob, in state `NotStarted`, JWKConsensusManager should reply to PRC request with errors without any state changes.
    let bob_ob_req = new_rpc_observation_request(999, issuer_bob.clone(), addrs[3], rpc_response_collector.clone());
    let handle_result = jwk_manager.process_peer_request(bob_ob_req);
    assert!(handle_result.is_ok());
    let last_invocations = std::mem::replace(&mut *rpc_response_collector.write(), vec![]);
    assert!(last_invocations.len() == 1 && last_invocations[0].is_err());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // Same for issuer Carl, except that a default entry for carl should be created on the fly.
    let carl_ob_req = new_rpc_observation_request(999, issuer_carl.clone(), addrs[3], rpc_response_collector.clone());
    let handle_result = jwk_manager.process_peer_request(carl_ob_req);
    assert!(handle_result.is_ok());
    let last_invocations = std::mem::replace(&mut *rpc_response_collector.write(), vec![]);
    assert!(last_invocations.len() == 1 && last_invocations[0].is_err());
    expected_states.insert(issuer_carl.clone(), PerProviderState {
        on_chain: None,
        observed: None,
        consensus_state: ConsensusState::NotStarted,
    });
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // For issuer Bob, in state `NotStarted`, JWKConsensusManager should do nothing if the observation is equal to on-chain state.
    let handle_result = jwk_manager.process_new_observation(issuer_bob.clone(), bob_jwks.clone());
    assert!(handle_result.is_ok());
    expected_states.get_mut(&issuer_bob).unwrap().observed = Some(bob_jwks.clone());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // Say now Alice retires a JWK.
    let alice_jwks_new = vec![
        JWK::new_unsupported(UnsupportedJWK::new_for_test("alice_jwk_id_1", "jwk_payload_1")),
    ];

    // For issuer Alice, in state `NotStarted`, JWKConsensusManager should initiate a JWK consensus session if an update was observed.
    let handle_result = jwk_manager.process_new_observation(issuer_alice.clone(), alice_jwks_new.clone());
    assert!(handle_result.is_ok());
    {
        let expected_alice_state = expected_states.get_mut(&issuer_alice).unwrap();
        expected_alice_state.observed = Some(alice_jwks_new.clone());
        let payload = ProviderJWKs {
            issuer: issuer_alice.clone(),
            version: 112,
            jwks: alice_jwks_new.clone(),
        };
        let signature = private_keys[0].sign(&payload).unwrap();
        expected_alice_state.consensus_state = ConsensusState::InProgress { my_proposal: ObservedUpdate {
            author: addrs[0],
            payload,
            signature,
        }, abort_handle_wrapper: AbortHandleWrapper::dummy() };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // If we also found a JWK update for issuer Carl, a separate JWK consensus session should be started.
    let carl_jwks_new = vec![
        JWK::new_unsupported(UnsupportedJWK::new_for_test("carl_jwk_id_0", "carl_jwk_payload_0")),
    ];
    let handle_result = jwk_manager.process_new_observation(issuer_carl.clone(), carl_jwks_new.clone());
    assert!(handle_result.is_ok());
    {
        let expected_carl_state = expected_states.get_mut(&issuer_carl).unwrap();
        expected_carl_state.observed = Some(carl_jwks_new.clone());
        let payload = ProviderJWKs {
            issuer: issuer_carl.clone(),
            version: 1,
            jwks: carl_jwks_new.clone(),
        };
        let signature = private_keys[0].sign(&payload).unwrap();
        expected_carl_state.consensus_state = ConsensusState::InProgress { my_proposal: ObservedUpdate {
            author: addrs[0],
            payload,
            signature,
        }, abort_handle_wrapper: AbortHandleWrapper::dummy() };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // Now that there are in-progress consensus sessions for Alice/Carl,
    // if receiving an observation request for issuer Alice/Carl, JWKConsensusManager should reply with their signed observation.
    let alice_ob_req = new_rpc_observation_request(999, issuer_alice.clone(), addrs[3], rpc_response_collector.clone());
    let carl_ob_req = new_rpc_observation_request(999, issuer_carl.clone(), addrs[3], rpc_response_collector.clone());
    let handle_result = jwk_manager.process_peer_request(alice_ob_req);
    assert!(handle_result.is_ok());
    let handle_result = jwk_manager.process_peer_request(carl_ob_req);
    assert!(handle_result.is_ok());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let last_invocations: Vec<JWKConsensusMsg> = std::mem::replace(&mut *rpc_response_collector.write(), vec![])
        .into_iter().map(|maybe_msg| maybe_msg.unwrap()).collect();
    let expected_responses = vec![
        JWKConsensusMsg::ObservationResponse(ObservedUpdateResponse { update: expected_states.get(&issuer_alice).unwrap().consensus_state.my_proposal_cloned() }),
        JWKConsensusMsg::ObservationResponse(ObservedUpdateResponse { update: expected_states.get(&issuer_carl).unwrap().consensus_state.my_proposal_cloned() }),
    ];
    assert_eq!(expected_responses, last_invocations);

    // If Alice rotates again while the consensus session for Alice is in progress, the existing session should be discarded and a new session should start.
    let alice_jwks_new_2 = vec![
        JWK::new_unsupported(UnsupportedJWK::new_for_test("alice_jwk_id_1", "jwk_payload_1")),
        JWK::new_unsupported(UnsupportedJWK::new_for_test("alice_jwk_id_3", "jwk_payload_3")),
    ];
    let handle_result = jwk_manager.process_new_observation(issuer_alice.clone(), alice_jwks_new_2.clone());
    assert!(handle_result.is_ok());
    {
        let expected_alice_state = expected_states.get_mut(&issuer_alice).unwrap();
        expected_alice_state.observed = Some(alice_jwks_new_2.clone());
        let payload = ProviderJWKs {
            issuer: issuer_alice.clone(),
            version: 112,
            jwks: alice_jwks_new_2.clone(),
        };
        let signature = private_keys[0].sign(&payload).unwrap();
        expected_alice_state.consensus_state = ConsensusState::InProgress { my_proposal: ObservedUpdate {
            author: addrs[0],
            payload,
            signature,
        }, abort_handle_wrapper: AbortHandleWrapper::dummy() };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // For issuer Carl, in state `InProgress`, when receiving a quorum-certified update from the the aggregator:
    // the state should be switched to `Finished`;
    // Carl's update should be available in validator txn pool.
    let qc_jwks_for_carl = expected_states.get(&issuer_carl).unwrap().consensus_state.my_proposal_cloned().payload;
    let multi_sig = Signature::aggregate(private_keys.iter().map(|sk|sk.sign(&qc_jwks_for_carl).unwrap()).collect::<Vec<_>>()).unwrap();
    let qc_update_for_carl = QuorumCertifiedUpdate {
        authors: BTreeSet::from_iter(addrs.clone()),
        payload: qc_jwks_for_carl,
        multi_sig,
    };
    let handle_result = jwk_manager.process_quorum_certified_update(qc_update_for_carl.clone());
    assert!(handle_result.is_ok());
    {
        let expected_carl_state = expected_states.get_mut(&issuer_carl).unwrap();
        expected_carl_state.consensus_state = ConsensusState::Finished {
            my_proposal: expected_carl_state.consensus_state.my_proposal_cloned(),
            quorum_certified: qc_update_for_carl.clone(),
        };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let expected_vtxns = vec![ValidatorTransaction::ObservedJWKsUpdates { updates: BTreeMap::from_iter(vec![(issuer_carl.clone(), qc_update_for_carl.clone())]) }];
    let actual_vtxns = vtxn_pool_read_cli.pull(Duration::from_secs(3600), 999, 2048, TransactionFilter::empty()).await;
    assert_eq!(expected_vtxns, actual_vtxns);

    // For issuer Carl, in state 'Finished`, JWKConsensusManager should still reply to observation requests with its own proposal.
    let carl_ob_req = new_rpc_observation_request(999, issuer_carl.clone(), addrs[3], rpc_response_collector.clone());
    let handle_result = jwk_manager.process_peer_request(carl_ob_req);
    assert!(handle_result.is_ok());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let expected_responses = vec![
        JWKConsensusMsg::ObservationResponse(ObservedUpdateResponse { update: expected_states.get(&issuer_carl).unwrap().consensus_state.my_proposal_cloned() }),
    ];
    let actual_responses: Vec<JWKConsensusMsg> = std::mem::replace(&mut *rpc_response_collector.write(), vec![])
        .into_iter().map(|maybe_msg| maybe_msg.unwrap()).collect();
    assert_eq!(expected_responses, actual_responses);


    // If the consensus session for Alice is also done, JWKConsensusManager should:
    // update the state for Alice to `Finished`;
    // update the validator txn in the pool to also include the update for Alice.
    let qc_jwks_for_alice = expected_states.get(&issuer_alice).unwrap().consensus_state.my_proposal_cloned().payload;
    let multi_sig = Signature::aggregate(private_keys.iter().take(3).map(|sk|sk.sign(&qc_jwks_for_alice).unwrap()).collect::<Vec<_>>()).unwrap();
    let qc_update_for_alice = QuorumCertifiedUpdate {
        authors: BTreeSet::from_iter(addrs[0..3].to_vec()),
        payload: qc_jwks_for_alice,
        multi_sig,
    };
    let handle_result = jwk_manager.process_quorum_certified_update(qc_update_for_alice.clone());
    assert!(handle_result.is_ok());
    {
        let expected_alice_state = expected_states.get_mut(&issuer_alice).unwrap();
        expected_alice_state.consensus_state = ConsensusState::Finished {
            my_proposal: expected_alice_state.consensus_state.my_proposal_cloned(),
            quorum_certified: qc_update_for_alice.clone(),
        };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let expected_vtxns = vec![ValidatorTransaction::ObservedJWKsUpdates { updates: BTreeMap::from_iter(vec![
        (issuer_alice.clone(), qc_update_for_alice),
        (issuer_carl.clone(), qc_update_for_carl),
    ]) }];
    let actual_vtxns = vtxn_pool_read_cli.pull(Duration::from_secs(3600), 999, 2048, TransactionFilter::empty()).await;
    assert_eq!(expected_vtxns, actual_vtxns);

    // At any time, JWKConsensusManager should fully follow on-chain update notification and re-initialize.
    let on_chain_state_carl_v1 = ProviderJWKs {
        issuer: issuer_carl.clone(),
        version: 1,
        jwks: carl_jwks_new.clone(),
    };
    let second_on_chain_state = ObservedJWKs {
        jwks: JWKs { entries: vec![
            on_chain_state_carl_v1.clone(),
        ]},
    };

    let handle_result = jwk_manager.reset_with_on_chain_state(second_on_chain_state);
    assert!(handle_result.is_ok());
    let mut expected_states = HashMap::from([
        (issuer_carl.clone(), PerProviderState {
            on_chain: Some(on_chain_state_carl_v1.clone()),
            observed: None,
            consensus_state: ConsensusState::NotStarted,
        }),
    ]);
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
}

#[cfg(test)]
fn new_rpc_observation_request(
    epoch: u64,
    issuer: Issuer,
    sender: AccountAddress,
    response_collector: Arc<RwLock<Vec<Result<JWKConsensusMsg>>>>,
) -> IncomingRpcRequest {
    IncomingRpcRequest {
        msg: JWKConsensusMsg::ObservationRequest(ObservedUpdateRequest { issuer }),
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(response_collector)),
    }
}
