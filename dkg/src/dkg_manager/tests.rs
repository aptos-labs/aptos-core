// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    agg_trx_producer::DummyAggTranscriptProducer,
    dkg_manager::{DKGManager, InnerState},
    network::{DummyRpcResponseSender, IncomingRpcRequest},
    types::DKGTranscriptRequest,
    DKGMessage,
};
use velor_crypto::{
    bls12381::{PrivateKey, PublicKey},
    Uniform,
};
use velor_infallible::RwLock;
use velor_types::{
    dkg::{
        dummy_dkg::DummyDKG, DKGSessionMetadata, DKGStartEvent, DKGTrait, DKGTranscript,
        DKGTranscriptMetadata,
    },
    epoch_state::EpochState,
    on_chain_config::OnChainRandomnessConfig,
    validator_txn::ValidatorTransaction,
    validator_verifier::{
        ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
    },
};
use velor_validator_transaction_pool::{TransactionFilter, VTxnPoolState};
use move_core_types::account_address::AccountAddress;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
async fn test_dkg_state_transition() {
    // Setup a validator set of 4 validators.
    let private_keys: Vec<Arc<PrivateKey>> = (0..4)
        .map(|_| Arc::new(PrivateKey::generate_for_testing()))
        .collect();
    let public_keys: Vec<PublicKey> = private_keys
        .iter()
        .map(|sk| PublicKey::from(sk.as_ref()))
        .collect();
    let addrs: Vec<AccountAddress> = (0..4).map(|_| AccountAddress::random()).collect();
    let voting_powers: Vec<u64> = vec![1, 1, 1, 1];
    let vtxn_pool_handle = VTxnPoolState::default();
    let validator_consensus_infos: Vec<ValidatorConsensusInfo> = (0..4)
        .map(|i| ValidatorConsensusInfo::new(addrs[i], public_keys[i].clone(), voting_powers[i]))
        .collect();
    let validator_consensus_info_move_structs = validator_consensus_infos
        .clone()
        .into_iter()
        .map(ValidatorConsensusInfoMoveStruct::from)
        .collect::<Vec<_>>();
    let epoch_state = EpochState {
        epoch: 999,
        verifier: Arc::new(ValidatorVerifier::new(validator_consensus_infos.clone())),
    };
    let agg_node_producer = DummyAggTranscriptProducer {};
    let mut dkg_manager: DKGManager<DummyDKG> = DKGManager::new(
        private_keys[0].clone(),
        0,
        addrs[0],
        Arc::new(epoch_state),
        Arc::new(agg_node_producer),
        vtxn_pool_handle.clone(),
    );

    // Initial state should be `NotStarted`.
    assert!(matches!(&dkg_manager.state, InnerState::NotStarted));

    let rpc_response_collector = Arc::new(RwLock::new(vec![]));

    // In state `NotStarted`, DKGManager should reply to RPC request with errors.
    let rpc_node_request = new_rpc_node_request(999, addrs[3], rpc_response_collector.clone());
    let handle_result = dkg_manager.process_peer_rpc_msg(rpc_node_request).await;
    assert!(handle_result.is_ok());
    let last_invocations = std::mem::take(&mut *rpc_response_collector.write());
    assert!(last_invocations.len() == 1 && last_invocations[0].is_err());
    assert!(matches!(&dkg_manager.state, InnerState::NotStarted));

    // In state `NotStarted`, DKGManager should accept `DKGStartEvent`:
    // it should record start time, compute its own node, and enter state `InProgress`.
    let start_time_1 = Duration::from_secs(1700000000);
    let event = DKGStartEvent {
        session_metadata: DKGSessionMetadata {
            dealer_epoch: 999,
            randomness_config: OnChainRandomnessConfig::default_enabled().into(),
            dealer_validator_set: validator_consensus_info_move_structs.clone(),
            target_validator_set: validator_consensus_info_move_structs.clone(),
        },
        start_time_us: start_time_1.as_micros() as u64,
    };
    let handle_result = dkg_manager.process_dkg_start_event(event.clone()).await;
    assert!(handle_result.is_ok());
    assert!(
        matches!(&dkg_manager.state, InnerState::InProgress { start_time, my_transcript, .. } if *start_time == start_time_1 && my_transcript.metadata == DKGTranscriptMetadata{ epoch: 999, author: addrs[0]})
    );

    // 2nd `DKGStartEvent` should be rejected.
    let handle_result = dkg_manager.process_dkg_start_event(event).await;
    println!("{:?}", handle_result);
    assert!(handle_result.is_err());

    // In state `InProgress`, DKGManager should respond to `DKGNodeRequest` with its own node.
    let rpc_node_request = new_rpc_node_request(999, addrs[3], rpc_response_collector.clone());
    let handle_result = dkg_manager.process_peer_rpc_msg(rpc_node_request).await;
    assert!(handle_result.is_ok());
    let last_responses = std::mem::take(&mut *rpc_response_collector.write())
        .into_iter()
        .map(anyhow::Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        vec![DKGMessage::TranscriptResponse(
            dkg_manager.state.my_node_cloned()
        )],
        last_responses
    );
    assert!(matches!(&dkg_manager.state, InnerState::InProgress { .. }));

    // In state `InProgress`, DKGManager should accept `DKGAggNode`:
    // it should update validator txn pool, and enter state `Finished`.
    let agg_trx = <DummyDKG as DKGTrait>::Transcript::default();
    let handle_result = dkg_manager
        .process_aggregated_transcript(agg_trx.clone())
        .await;
    assert!(handle_result.is_ok());
    let available_vtxns = vtxn_pool_handle.pull(
        Instant::now() + Duration::from_secs(10),
        999,
        2048,
        TransactionFilter::no_op(),
    );
    assert_eq!(
        vec![ValidatorTransaction::DKGResult(DKGTranscript {
            metadata: DKGTranscriptMetadata {
                epoch: 999,
                author: addrs[0],
            },
            transcript_bytes: bcs::to_bytes(&agg_trx).unwrap(),
        })],
        available_vtxns
    );
    assert!(matches!(&dkg_manager.state, InnerState::Finished { .. }));

    // In state `Finished`, DKGManager should still respond to `DKGNodeRequest` with its own node.
    let rpc_node_request = new_rpc_node_request(999, addrs[3], rpc_response_collector.clone());
    let handle_result = dkg_manager.process_peer_rpc_msg(rpc_node_request).await;
    assert!(handle_result.is_ok());
    let last_responses = std::mem::take(&mut *rpc_response_collector.write())
        .into_iter()
        .map(anyhow::Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        vec![DKGMessage::TranscriptResponse(
            dkg_manager.state.my_node_cloned()
        )],
        last_responses
    );
    assert!(matches!(&dkg_manager.state, InnerState::Finished { .. }));
}

#[cfg(test)]
fn new_rpc_node_request(
    epoch: u64,
    sender: AccountAddress,
    response_collector: Arc<RwLock<Vec<anyhow::Result<DKGMessage>>>>,
) -> IncomingRpcRequest {
    IncomingRpcRequest {
        msg: DKGMessage::TranscriptRequest(DKGTranscriptRequest::new(epoch)),
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(response_collector)),
    }
}
