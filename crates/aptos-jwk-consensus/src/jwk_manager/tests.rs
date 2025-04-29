// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwk_manager::{
        ConsensusState, IssuerLevelConsensusManager, PerProviderState, QuorumCertProcessGuard,
    },
    mode::TConsensusMode,
    network::{DummyRpcResponseSender, IncomingRpcRequest},
    types::{JWKConsensusMsg, ObservedUpdate, ObservedUpdateRequest, ObservedUpdateResponse},
    update_certifier::TUpdateCertifier,
};
use aptos_bitvec::BitVec;
use aptos_channels::aptos_channel;
use aptos_crypto::{
    bls12381::{PrivateKey, PublicKey, Signature},
    hash::CryptoHash,
    SigningKey, Uniform,
};
use aptos_infallible::{Mutex, RwLock};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    epoch_state::EpochState,
    jwks::{
        issuer_from_str, jwk::JWK, unsupported::UnsupportedJWK, AllProvidersJWKs, Issuer,
        ProviderJWKs, QuorumCertifiedUpdate,
    },
    validator_txn::ValidatorTransaction,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use aptos_validator_transaction_pool::{TransactionFilter, VTxnPoolState};
use futures_util::future::AbortHandle;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
async fn test_jwk_manager_state_transition() {
    // Setting up an epoch of 4 validators, and simulate the JWKManager in validator 0.
    let private_keys: Vec<Arc<PrivateKey>> = (0..4)
        .map(|_| Arc::new(PrivateKey::generate_for_testing()))
        .collect();
    let public_keys: Vec<PublicKey> = private_keys
        .iter()
        .map(|sk| PublicKey::from(sk.as_ref()))
        .collect();
    let addrs: Vec<AccountAddress> = (0..4).map(|_| AccountAddress::random()).collect();
    let voting_powers: Vec<u64> = vec![1, 1, 1, 1];
    let validator_consensus_infos: Vec<ValidatorConsensusInfo> = (0..4)
        .map(|i| ValidatorConsensusInfo::new(addrs[i], public_keys[i].clone(), voting_powers[i]))
        .collect();
    let epoch_state = EpochState {
        epoch: 999,
        verifier: ValidatorVerifier::new(validator_consensus_infos.clone()).into(),
    };

    let update_certifier = DummyUpdateCertifier::default();
    let vtxn_pool = VTxnPoolState::default();
    let mut jwk_manager = IssuerLevelConsensusManager::new(
        private_keys[0].clone(),
        addrs[0],
        Arc::new(epoch_state),
        Arc::new(update_certifier),
        vtxn_pool.clone(),
    );

    // In this example, Alice and Bob are 2 existing issuers; Carl was added in the last epoch so no JWKs of Carl is on chain.
    let issuer_alice = issuer_from_str("https://alice.info");
    let issuer_bob = issuer_from_str("https://bob.io");
    let issuer_carl = issuer_from_str("https://carl.dev");
    let alice_jwks = vec![
        JWK::Unsupported(UnsupportedJWK::new_for_testing(
            "alice_jwk_id_0",
            "jwk_payload_0",
        ))
        .into(),
        JWK::Unsupported(UnsupportedJWK::new_for_testing(
            "alice_jwk_id_1",
            "jwk_payload_1",
        ))
        .into(),
    ];
    let bob_jwks = vec![
        JWK::Unsupported(UnsupportedJWK::new_for_testing(
            "bob_jwk_id_0",
            "jwk_payload_2",
        ))
        .into(),
        JWK::Unsupported(UnsupportedJWK::new_for_testing(
            "bob_jwk_id_1",
            "jwk_payload_3",
        ))
        .into(),
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

    let initial_on_chain_state = AllProvidersJWKs {
        entries: vec![
            on_chain_state_alice_v111.clone(),
            on_chain_state_bob_v222.clone(),
        ],
    };

    // On start, JWKManager is always initialized with the on-chain state.
    assert!(jwk_manager
        .reset_with_on_chain_state(initial_on_chain_state)
        .is_ok());
    let mut expected_states = HashMap::from([
        (
            issuer_alice.clone(),
            PerProviderState::new(on_chain_state_alice_v111.clone()),
        ),
        (
            issuer_bob.clone(),
            PerProviderState::new(on_chain_state_bob_v222.clone()),
        ),
    ]);
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    let rpc_response_collector = Arc::new(RwLock::new(vec![]));

    // When JWK consensus is `NotStarted` for issuer Bob, JWKConsensusManager should:
    // reply an error to any observation request and keep the state unchanged.
    let bob_ob_req = new_rpc_observation_request(
        999,
        issuer_bob.clone(),
        addrs[3],
        rpc_response_collector.clone(),
    );
    assert!(jwk_manager.process_peer_request(bob_ob_req).is_ok());
    let last_invocations = std::mem::take(&mut *rpc_response_collector.write());
    assert!(last_invocations.len() == 1 && last_invocations[0].is_err());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // When JWK consensus is `NotStarted` for issuer Carl, JWKConsensusManager should:
    // reply an error to any observation request and keep the state unchanged;
    // also create an entry in the state table on the fly.
    let carl_ob_req = new_rpc_observation_request(
        999,
        issuer_carl.clone(),
        addrs[3],
        rpc_response_collector.clone(),
    );
    assert!(jwk_manager.process_peer_request(carl_ob_req).is_ok());
    let last_invocations = std::mem::take(&mut *rpc_response_collector.write());
    assert!(last_invocations.len() == 1 && last_invocations[0].is_err());
    expected_states.insert(issuer_carl.clone(), PerProviderState::default());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // When JWK consensus is `NotStarted` for issuer Bob, JWKConsensusManager should:
    // do nothing to an observation equal to on-chain state (except storing it, which may be unnecessary).
    assert!(jwk_manager
        .process_new_observation(issuer_bob.clone(), bob_jwks.clone())
        .is_ok());
    expected_states.get_mut(&issuer_bob).unwrap().observed = Some(bob_jwks.clone());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // When JWK consensus is `NotStarted` for issuer Alice, JWKConsensusManager should:
    // initiate a JWK consensus session if an update was observed.
    let alice_jwks_new = vec![JWK::Unsupported(UnsupportedJWK::new_for_testing(
        "alice_jwk_id_1",
        "jwk_payload_1",
    ))
    .into()];
    assert!(jwk_manager
        .process_new_observation(issuer_alice.clone(), alice_jwks_new.clone())
        .is_ok());
    {
        let expected_alice_state = expected_states.get_mut(&issuer_alice).unwrap();
        expected_alice_state.observed = Some(alice_jwks_new.clone());
        let observed = ProviderJWKs {
            issuer: issuer_alice.clone(),
            version: 112, // on-chain baseline is at version 111.
            jwks: alice_jwks_new.clone(),
        };
        let signature = private_keys[0].sign(&observed).unwrap();
        expected_alice_state.consensus_state = ConsensusState::InProgress {
            my_proposal: ObservedUpdate {
                author: addrs[0],
                observed,
                signature,
            },
            abort_handle_wrapper: QuorumCertProcessGuard::dummy(),
        };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // If we also found a JWK update for issuer Carl, a separate JWK consensus session should be started.
    let carl_jwks_new = vec![JWK::Unsupported(UnsupportedJWK::new_for_testing(
        "carl_jwk_id_0",
        "jwk_payload_4",
    ))
    .into()];
    assert!(jwk_manager
        .process_new_observation(issuer_carl.clone(), carl_jwks_new.clone())
        .is_ok());
    {
        let expected_carl_state = expected_states.get_mut(&issuer_carl).unwrap();
        expected_carl_state.observed = Some(carl_jwks_new.clone());
        let observed = ProviderJWKs {
            issuer: issuer_carl.clone(),
            version: 1,
            jwks: carl_jwks_new.clone(),
        };
        let signature = private_keys[0].sign(&observed).unwrap();
        expected_carl_state.consensus_state = ConsensusState::InProgress {
            my_proposal: ObservedUpdate {
                author: addrs[0],
                observed,
                signature,
            },
            abort_handle_wrapper: QuorumCertProcessGuard::dummy(),
        };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // Now that there are in-progress consensus sessions for Alice/Carl,
    // if receiving an observation request for issuer Alice/Carl, JWKConsensusManager should reply with their signed observation.
    let alice_ob_req = new_rpc_observation_request(
        999,
        issuer_alice.clone(),
        addrs[3],
        rpc_response_collector.clone(),
    );
    let carl_ob_req = new_rpc_observation_request(
        999,
        issuer_carl.clone(),
        addrs[3],
        rpc_response_collector.clone(),
    );
    assert!(jwk_manager.process_peer_request(alice_ob_req).is_ok());
    assert!(jwk_manager.process_peer_request(carl_ob_req).is_ok());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let last_invocations: Vec<JWKConsensusMsg> =
        std::mem::take(&mut *rpc_response_collector.write())
            .into_iter()
            .map(|maybe_msg| maybe_msg.unwrap())
            .collect();
    let expected_responses = vec![
        JWKConsensusMsg::ObservationResponse(ObservedUpdateResponse {
            epoch: 999,
            update: expected_states
                .get(&issuer_alice)
                .unwrap()
                .consensus_state
                .my_proposal_cloned(),
        }),
        JWKConsensusMsg::ObservationResponse(ObservedUpdateResponse {
            epoch: 999,
            update: expected_states
                .get(&issuer_carl)
                .unwrap()
                .consensus_state
                .my_proposal_cloned(),
        }),
    ];
    assert_eq!(expected_responses, last_invocations);

    // If Alice rotates again while the consensus session for Alice is in progress, the existing session should be discarded and a new session should start.
    let alice_jwks_new_2 = vec![
        JWK::Unsupported(UnsupportedJWK::new_for_testing(
            "alice_jwk_id_1",
            "jwk_payload_1",
        ))
        .into(),
        JWK::Unsupported(UnsupportedJWK::new_for_testing(
            "alice_jwk_id_3",
            "jwk_payload_5",
        ))
        .into(),
    ];
    assert!(jwk_manager
        .process_new_observation(issuer_alice.clone(), alice_jwks_new_2.clone())
        .is_ok());
    {
        let expected_alice_state = expected_states.get_mut(&issuer_alice).unwrap();
        expected_alice_state.observed = Some(alice_jwks_new_2.clone());
        let observed = ProviderJWKs {
            issuer: issuer_alice.clone(),
            version: 112,
            jwks: alice_jwks_new_2.clone(),
        };
        let signature = private_keys[0].sign(&observed).unwrap();
        expected_alice_state.consensus_state = ConsensusState::InProgress {
            my_proposal: ObservedUpdate {
                author: addrs[0],
                observed,
                signature,
            },
            abort_handle_wrapper: QuorumCertProcessGuard::dummy(),
        };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);

    // For issuer Carl, in state `InProgress`, when receiving a quorum-certified update from the aggregator:
    // the state should be switched to `Finished`;
    // Carl's update should be available in validator txn pool.
    let qc_jwks_for_carl = expected_states
        .get(&issuer_carl)
        .unwrap()
        .consensus_state
        .my_proposal_cloned()
        .observed;
    let signer_bit_vec = BitVec::from(private_keys.iter().map(|_| true).collect::<Vec<_>>());
    let sig = Signature::aggregate(
        private_keys
            .iter()
            .map(|sk| sk.sign(&qc_jwks_for_carl).unwrap())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let multi_sig = AggregateSignature::new(signer_bit_vec, Some(sig));
    let qc_update_for_carl = QuorumCertifiedUpdate {
        update: qc_jwks_for_carl,
        multi_sig,
    };
    assert!(jwk_manager
        .process_quorum_certified_update(qc_update_for_carl.clone())
        .is_ok());
    {
        let expected_carl_state = expected_states.get_mut(&issuer_carl).unwrap();
        expected_carl_state.consensus_state = ConsensusState::Finished {
            vtxn_guard: vtxn_pool.dummy_txn_guard(),
            my_proposal: expected_carl_state.consensus_state.my_proposal_cloned(),
            quorum_certified: qc_update_for_carl.clone(),
        };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let expected_vtxns = vec![ValidatorTransaction::ObservedJWKUpdate(
        qc_update_for_carl.clone(),
    )];
    let actual_vtxns = vtxn_pool.pull(
        Instant::now() + Duration::from_secs(3600),
        999,
        2048,
        TransactionFilter::empty(),
    );
    assert_eq!(expected_vtxns, actual_vtxns);

    // For issuer Carl, in state 'Finished`, JWKConsensusManager should still reply to observation requests with its own proposal.
    let carl_ob_req = new_rpc_observation_request(
        999,
        issuer_carl.clone(),
        addrs[3],
        rpc_response_collector.clone(),
    );
    assert!(jwk_manager.process_peer_request(carl_ob_req).is_ok());
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let expected_responses = vec![JWKConsensusMsg::ObservationResponse(
        ObservedUpdateResponse {
            epoch: 999,
            update: expected_states
                .get(&issuer_carl)
                .unwrap()
                .consensus_state
                .my_proposal_cloned(),
        },
    )];
    let actual_responses: Vec<JWKConsensusMsg> =
        std::mem::take(&mut *rpc_response_collector.write())
            .into_iter()
            .map(|maybe_msg| maybe_msg.unwrap())
            .collect();
    assert_eq!(expected_responses, actual_responses);

    // If the consensus session for Alice is also done, JWKConsensusManager should:
    // update the state for Alice to `Finished`;
    // update the validator txn in the pool to also include the update for Alice.
    let qc_jwks_for_alice = expected_states
        .get(&issuer_alice)
        .unwrap()
        .consensus_state
        .my_proposal_cloned()
        .observed;
    let signer_bit_vec = BitVec::from(
        private_keys
            .iter()
            .take(3)
            .map(|_| true)
            .collect::<Vec<_>>(),
    );
    let sig = Signature::aggregate(
        private_keys
            .iter()
            .take(3)
            .map(|sk| sk.sign(&qc_jwks_for_alice).unwrap())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let multi_sig = AggregateSignature::new(signer_bit_vec, Some(sig));
    let qc_update_for_alice = QuorumCertifiedUpdate {
        update: qc_jwks_for_alice,
        multi_sig,
    };
    assert!(jwk_manager
        .process_quorum_certified_update(qc_update_for_alice.clone())
        .is_ok());
    {
        let expected_alice_state = expected_states.get_mut(&issuer_alice).unwrap();
        expected_alice_state.consensus_state = ConsensusState::Finished {
            vtxn_guard: vtxn_pool.dummy_txn_guard(),
            my_proposal: expected_alice_state.consensus_state.my_proposal_cloned(),
            quorum_certified: qc_update_for_alice.clone(),
        };
    }
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
    let expected_vtxn_hashes = vec![
        ValidatorTransaction::ObservedJWKUpdate(qc_update_for_alice),
        ValidatorTransaction::ObservedJWKUpdate(qc_update_for_carl),
    ]
    .iter()
    .map(CryptoHash::hash)
    .collect::<HashSet<_>>();

    let actual_vtxn_hashes = vtxn_pool
        .pull(
            Instant::now() + Duration::from_secs(3600),
            999,
            2048,
            TransactionFilter::empty(),
        )
        .iter()
        .map(CryptoHash::hash)
        .collect::<HashSet<_>>();
    assert_eq!(expected_vtxn_hashes, actual_vtxn_hashes);

    // At any time, JWKConsensusManager should fully follow on-chain update notification and re-initialize.
    let second_on_chain_state = AllProvidersJWKs {
        entries: vec![on_chain_state_alice_v111.clone()],
    };

    assert!(jwk_manager
        .reset_with_on_chain_state(second_on_chain_state)
        .is_ok());
    expected_states.remove(&issuer_bob);
    expected_states.remove(&issuer_carl);
    assert_eq!(expected_states, jwk_manager.states_by_issuer);
}

fn new_rpc_observation_request(
    epoch: u64,
    issuer: Issuer,
    sender: AccountAddress,
    response_collector: Arc<RwLock<Vec<anyhow::Result<JWKConsensusMsg>>>>,
) -> IncomingRpcRequest {
    IncomingRpcRequest {
        msg: JWKConsensusMsg::ObservationRequest(ObservedUpdateRequest { epoch, issuer }),
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(response_collector)),
    }
}

pub struct DummyUpdateCertifier {
    pub invocations: Mutex<Vec<(Arc<EpochState>, ProviderJWKs)>>,
}

impl Default for DummyUpdateCertifier {
    fn default() -> Self {
        Self {
            invocations: Mutex::new(vec![]),
        }
    }
}

impl<ConsensusMode: TConsensusMode> TUpdateCertifier<ConsensusMode> for DummyUpdateCertifier {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        _agg_node_tx: aptos_channel::Sender<
            ConsensusMode::ConsensusSessionKey,
            QuorumCertifiedUpdate,
        >,
    ) -> anyhow::Result<AbortHandle> {
        self.invocations.lock().push((epoch_state, payload));
        let (abort_handle, _) = AbortHandle::new_pair();
        Ok(abort_handle)
    }
}
