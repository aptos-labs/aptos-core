// Copyright © Aptos Foundation

use crate::{
    agg_trx_producer::DummyAggTranscriptProducer,
    dkg_manager::{DKGManager, InnerState},
    dummy_dkg::{DummyDKG, DummyDKGTranscript},
    network::{DummyRpcResponseSender, IncomingRpcRequest},
    types::DKGNodeRequest,
    DKGMessage,
};
use aptos_crypto::{
    bls12381::{PrivateKey, PublicKey},
    Uniform,
};
use aptos_infallible::RwLock;
use aptos_types::{
    dkg::{DKGNode, DKGStartEvent, DKGTrait, DKGTranscriptMetadata},
    epoch_state::EpochState,
    on_chain_config::ValidatorSet,
    validator_config::ValidatorConfig,
    validator_info::ValidatorInfo,
    validator_txn::{Topic, ValidatorTransaction},
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use aptos_validator_transaction_pool as vtxn_pool;
use aptos_validator_transaction_pool::TransactionFilter;
use move_core_types::account_address::AccountAddress;
use std::{sync::Arc, time::Duration};

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
    let (vtxn_read_client, mut vtxn_write_clients) = vtxn_pool::new(vec![(Topic::DKG, None)]);
    let vtxn_write_client = vtxn_write_clients.pop().unwrap();
    let validator_consensus_infos: Vec<ValidatorConsensusInfo> = (0..4)
        .map(|i| ValidatorConsensusInfo::new(addrs[i], public_keys[i].clone(), voting_powers[i]))
        .collect();
    let validator_configs: Vec<ValidatorConfig> = (0..4)
        .map(|i| ValidatorConfig::new(public_keys[i].clone(), vec![], vec![], i as u64))
        .collect();
    let validator_infos: Vec<ValidatorInfo> = (0..4)
        .map(|i| ValidatorInfo::new(addrs[i], voting_powers[i], validator_configs[i].clone()))
        .collect();
    let validator_set = ValidatorSet::new(validator_infos.clone());

    let epoch_state = EpochState {
        epoch: 999,
        verifier: ValidatorVerifier::new(validator_consensus_infos.clone()),
    };
    let agg_node_producer = DummyAggTranscriptProducer {};
    let mut dkg_manager = DKGManager::new(
        private_keys[0].clone(),
        addrs[0],
        Arc::new(epoch_state),
        Arc::new(agg_node_producer),
        Arc::new(vtxn_write_client),
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
    let handle_result = dkg_manager
        .process_dkg_start_event(DKGStartEvent {
            target_epoch: 1000,
            start_time_us: 1700000000000000,
            target_validator_set: validator_set.clone(), // No validator set change!
        })
        .await;
    assert!(handle_result.is_ok());
    assert!(
        matches!(&dkg_manager.state, InnerState::InProgress { start_time_us, my_node, .. } if *start_time_us == 1700000000000000 && my_node.metadata == DKGTranscriptMetadata{ epoch: 999, author: addrs[0]})
    );

    // In state `InProgress`, DKGManager should respond to `DKGNodeRequest` with its own node.
    let rpc_node_request = new_rpc_node_request(999, addrs[3], rpc_response_collector.clone());
    let handle_result = dkg_manager.process_peer_rpc_msg(rpc_node_request).await;
    assert!(handle_result.is_ok());
    let last_responses = std::mem::take(&mut *rpc_response_collector.write())
        .into_iter()
        .map(anyhow::Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        vec![DKGMessage::NodeResponse(dkg_manager.state.my_node_cloned())],
        last_responses
    );
    assert!(matches!(&dkg_manager.state, InnerState::InProgress { .. }));

    // In state `InProgress`, DKGManager should accept `DKGAggNode`:
    // it should update validator txn pool, and enter state `Finished`.
    let agg_trx = DummyDKGTranscript::default();
    let handle_result = dkg_manager
        .process_aggregated_transcript(agg_trx.clone())
        .await;
    assert!(handle_result.is_ok());
    let available_vtxns = vtxn_read_client
        .pull(
            Duration::from_secs(10),
            999,
            2048,
            TransactionFilter::no_op(),
        )
        .await;
    assert_eq!(
        vec![ValidatorTransaction::DKGResult(DKGNode {
            metadata: DKGTranscriptMetadata {
                epoch: 999,
                author: addrs[0],
            },
            transcript_bytes: DummyDKG::serialize_transcript(&agg_trx),
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
        vec![DKGMessage::NodeResponse(dkg_manager.state.my_node_cloned())],
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
        msg: DKGMessage::NodeRequest(DKGNodeRequest::new(epoch)),
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(response_collector)),
    }
}
