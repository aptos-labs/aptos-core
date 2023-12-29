// Copyright © Aptos Foundation

#[cfg(test)]
use crate::types::DKGNodeMetadata;
use crate::dkg_manager::agg_node_producer::AggNodeProducer;
#[cfg(test)]
use crate::dkg_manager::agg_node_producer::DummyAggNodeProducer;
use crate::network::DummyRpcResponseSender;
use crate::network::IncomingRpcRequest;
use crate::tracing::observe_dkg;
use crate::tracing::DKGStage;
use crate::types::DKGNodeRequest;
use crate::DKGMessage;
use crate::DKGNode;
use anyhow::{anyhow, bail, ensure, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::{
    bls12381::{PrivateKey, PublicKey},
    Uniform,
};
use aptos_dkg::pvss::{traits::Transcript, Player};
use aptos_infallible::RwLock;
use aptos_logger::{debug, error};
use aptos_types::{
    dkg::{
        build_dkg_pvss_config, DKGAggNode, DKGAggNodeMetadata, DKGPvssConfig, DKGTranscriptWrapper,
        StartDKGEvent, WTrx,
    },
    epoch_state::EpochState,
    on_chain_config::{DKGSessionState, ValidatorSet},
    validator_config::ValidatorConfig,
    validator_info::ValidatorInfo,
    validator_txn::{Topic, ValidatorTransaction},
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use aptos_validator_transaction_pool as vtxn_pool;
use aptos_validator_transaction_pool::TransactionFilter;
use fail::fail_point;
use futures::{future::AbortHandle, FutureExt, StreamExt};
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use rand::{rngs::StdRng, thread_rng, SeedableRng};
use std::{sync::Arc, time::Duration};

pub mod agg_node_producer;

#[derive(Clone, Debug)]
enum InnerState {
    NotStarted,
    InProgress {
        start_time_us: u64,
        pvss_config: DKGPvssConfig,
        my_node: DKGNode,
        abort_handle: AbortHandle,
    },
    Finished {
        start_time_us: u64,
        my_node: DKGNode,
    },
}

impl InnerState {
    fn variant_name(&self) -> &str {
        match self {
            InnerState::NotStarted => "NotStarted",
            InnerState::InProgress { .. } => "InProgress",
            InnerState::Finished { .. } => "Finished",
        }
    }

    #[cfg(test)]
    pub fn my_node_cloned(&self) -> DKGNode {
        match self {
            InnerState::NotStarted => panic!("my_node unavailable"),
            InnerState::InProgress { my_node, .. } | InnerState::Finished { my_node, .. } => {
                my_node.clone()
            },
        }
    }
}

pub struct DKGManager {
    my_addr: AccountAddress,
    epoch_state: EpochState,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    private_key: PrivateKey,
    agg_node_producer: Arc<dyn AggNodeProducer>,
    agg_node_tx: Option<aptos_channel::Sender<(), DKGAggNode>>,
    state: InnerState,
}

impl DKGManager {
    pub fn new(
        my_addr: AccountAddress,
        epoch_state: EpochState,
        private_key: PrivateKey,
        agg_node_producer: Arc<dyn AggNodeProducer>,
        vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    ) -> Self {
        Self {
            my_addr,
            epoch_state,
            vtxn_pool_write_cli,
            private_key,
            agg_node_tx: None,
            agg_node_producer,
            state: InnerState::NotStarted,
        }
    }

    pub async fn run(
        mut self,
        in_progress_session: Option<DKGSessionState>,
        mut start_dkg_event_rx: aptos_channel::Receiver<(), StartDKGEvent>,
        mut rpc_msg_rx: aptos_channel::Receiver<(), (AccountAddress, IncomingRpcRequest)>,
        mut dkg_txn_pulled_rx: vtxn_pool::PullNotificationReceiver,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        let (agg_node_tx, mut agg_node_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.agg_node_tx = Some(agg_node_tx);

        if let Some(session) = in_progress_session {
            self.setup_deal_broadcast(session.start_time_us, &session.target_validator_set)
                .await
                .unwrap();
        }

        let mut close_rx = close_rx.into_stream();
        loop {
            let handling_err = tokio::select! {
                start_dkg_event = start_dkg_event_rx.select_next_some() => {
                    self.process_start_dkg_event(start_dkg_event).await.err()
                },
                (_sender, msg) = rpc_msg_rx.select_next_some() => {
                    self.process_peer_rpc_msg(msg).await.err()
                },
                agg_node = agg_node_rx.select_next_some() => {
                    self.process_agg_node(agg_node).await.err()
                },
                dkg_txn = dkg_txn_pulled_rx.select_next_some() => {
                    self.process_dkg_txn_pulled_notification(dkg_txn).await.err()
                },
                close_req = close_rx.select_next_some() => {
                    debug!("[DKG] main: tearing down, epoch={}", self.epoch());
                    match &self.state {
                        InnerState::InProgress { start_time_us, .. } | InnerState::Finished { start_time_us, .. } => {
                            observe_dkg(Some(*start_time_us), DKGStage::DKG_FINISH);
                        },
                        _ => {},
                    }
                    self.vtxn_pool_write_cli.put(None);
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).unwrap();
                    }
                    break;
                }
            };

            if let Some(err) = handling_err {
                error!("[DKG] handling error: {err}");
            }

            debug!("[DKG] inner_state=={:?}", self.state);
        }
    }

    /// Process a locally formed AggNode.
    /// If we don't have an AggNode, accept the given one and forward it to validator txn pool.
    /// Otherwise, ignore it.
    /// Return whether the AggNode is accepted.
    async fn process_agg_node(&mut self, agg_node: DKGAggNode) -> Result<()> {
        debug!("[DKG] process_agg_node: BEGIN: epoch={}", self.epoch());
        fail_point!("dkg::process_agg_node");
        self.state = match &self.state {
            InnerState::InProgress {
                start_time_us,
                my_node,
                ..
            } => {
                observe_dkg(Some(*start_time_us), DKGStage::DKG_AGG_NODE_READY);
                let txn = ValidatorTransaction::DKGTranscriptForNextEpoch(agg_node);
                self.vtxn_pool_write_cli.put(Some(Arc::new(txn)));
                InnerState::Finished {
                    start_time_us: *start_time_us,
                    my_node: my_node.clone(),
                }
            },
            _ => bail!(
                "agg node only expected when production in progress, current state is {}",
                self.state.variant_name()
            ),
        };
        debug!("[DKG] process_agg_node: END");
        Ok(())
    }

    async fn process_dkg_txn_pulled_notification(
        &mut self,
        _txn: Arc<ValidatorTransaction>,
    ) -> Result<()> {
        match &self.state {
            InnerState::Finished { start_time_us, .. } => {
                observe_dkg(Some(*start_time_us), DKGStage::DKG_AGG_NODE_PROPOSED);
            },
            _ => {
                bail!("pull notification should only be delivered after dkg finished");
            },
        }
        Ok(())
    }

    /// Calculate DKG config. Deal a transcript. Start broadcasting the transcript.
    /// Called when a DKG start event is received, or when the node is restarting.
    ///
    /// NOTE: the dealt DKG transcript does not have to be persisted:
    /// it is ok for a validator to equivocate on its DKG transcript, as long as the transcript is valid.
    async fn setup_deal_broadcast(
        &mut self,
        start_time_us: u64,
        target_validator_set: &ValidatorSet,
    ) -> Result<()> {
        debug!("[DKG] setup_deal_broadcast: BEGIN, epoch={}", self.epoch());
        self.state = match &self.state {
            InnerState::NotStarted => {
                let dkg_pvss_config =
                    build_dkg_pvss_config(self.epoch_state.epoch, &target_validator_set);
                let my_index = *self
                    .epoch_state
                    .verifier
                    .address_to_validator_index()
                    .get(&self.my_addr)
                    .unwrap();

                let seed = if cfg!(feature = "smoke-test") {
                    debug!("[DKG] use smoke test special seed!");
                    // In DKG test, the test cases need to get the same input secret, so it can verify the reconstructed dealt secret.
                    // See function `verify_dkg_transcript()` in `testsuite/smoke-test/src/dkg/mod.rs`.
                    self.private_key.to_bytes()
                } else {
                    aptos_dkg::utils::random::random_scalar(&mut thread_rng()).to_bytes_le()
                };

                let mut rng = StdRng::from_seed(seed);

                // The secret generated by the dealer
                let s = <WTrx as Transcript>::InputSecret::generate(&mut rng);
                // The auxiliary information used for PVSS
                let aux = (self.epoch_state.epoch, self.my_addr);

                // compute one transcript for generating the keys for the randomness generation
                let trx = WTrx::deal(
                    &dkg_pvss_config.wconfig,
                    &dkg_pvss_config.pp,
                    &self.private_key,
                    &dkg_pvss_config.eks,
                    &s,
                    &aux,
                    &Player { id: my_index },
                    &mut rng,
                );

                let dkg_trx_wrapper = DKGTranscriptWrapper { trx };
                let dkg_node = DKGNode::new(
                    self.epoch_state.epoch,
                    self.my_addr,
                    dkg_trx_wrapper.clone(),
                );

                observe_dkg(Some(start_time_us), DKGStage::DKG_NODE_READY);

                let abort_handle = self.agg_node_producer.start_produce(
                    self.epoch_state.clone(),
                    dkg_pvss_config.clone(),
                    self.agg_node_tx.clone(),
                );

                InnerState::InProgress {
                    start_time_us,
                    pvss_config: dkg_pvss_config,
                    my_node: dkg_node,
                    abort_handle,
                }
            },
            _ => unreachable!(), // `setup_deal_broadcast` is called only when DKG state is `NotStarted`.
        };

        debug!("[DKG] setup_deal_broadcast: END");
        Ok(())
    }

    async fn process_start_dkg_event(&mut self, event: StartDKGEvent) -> Result<()> {
        let StartDKGEvent {
            target_epoch,
            start_time_us,
            target_validator_set,
        } = event;
        debug!(
            "[DKG] process_start_dkg_event: BEGIN: epoch={}",
            self.epoch()
        );
        fail_point!("dkg::process_start_dkg_event");
        ensure!(self.epoch_state.epoch + 1 == target_epoch);
        self.setup_deal_broadcast(start_time_us, &target_validator_set)
            .await?;
        debug!("[DKG] process_start_dkg_event: OK");
        Ok(())
    }

    async fn process_peer_rpc_msg(&mut self, req: IncomingRpcRequest) -> Result<()> {
        debug!("[DKG] process_peer_msg: BEGIN: epoch={}", self.epoch());
        let IncomingRpcRequest {
            msg,
            mut response_sender,
            ..
        } = req;
        ensure!(msg.epoch() == self.epoch());
        let response = match (&self.state, &msg) {
            (InnerState::Finished { my_node, .. }, DKGMessage::NodeRequest(_))
            | (InnerState::InProgress { my_node, .. }, DKGMessage::NodeRequest(_)) => {
                Ok(DKGMessage::NodeResponse(my_node.clone()))
            },
            _ => Err(anyhow!(
                "msg {:?} unexpected in state {:?}",
                msg.name(),
                self.state.variant_name()
            )),
        };

        response_sender.send(response);

        debug!("[DKG] process_peer_msg: END");
        Ok(())
    }

    fn epoch(&self) -> u64 {
        self.epoch_state.epoch
    }
}

#[tokio::test]
async fn test_dkg_state_transition() {
    // Setup a validator set of 4 validators.
    let private_keys: Vec<PrivateKey> =
        (0..4).map(|_| PrivateKey::generate_for_testing()).collect();
    let public_keys: Vec<PublicKey> = private_keys.iter().map(PublicKey::from).collect();
    let addrs: Vec<AccountAddress> = (0..4).map(|_| AccountAddress::random()).collect();
    let voting_powers: Vec<u64> = vec![1, 1, 1, 1];
    let (vtxn_read_client, mut vtxn_write_clients) =
        vtxn_pool::new(vec![(Topic::RANDOMNESS_DKG, None)]);
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
    let agg_node_producer = DummyAggNodeProducer::new();
    let mut dkg_manager = DKGManager::new(
        addrs[0],
        epoch_state,
        private_keys[0].clone(),
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
    let last_invocations = std::mem::replace(&mut *rpc_response_collector.write(), vec![]);
    assert!(last_invocations.len() == 1 && last_invocations[0].is_err());
    assert!(matches!(&dkg_manager.state, InnerState::NotStarted));

    // In state `NotStarted`, DKGManager should accept `DKGStartEvent`:
    // it should record start time, compute its own node, and enter state `InProgress`.
    let handle_result = dkg_manager
        .process_start_dkg_event(StartDKGEvent {
            target_epoch: 1000,
            start_time_us: 1700000000000000,
            target_validator_set: validator_set.clone(), // No validator set change!
        })
        .await;
    assert!(handle_result.is_ok());
    assert!(
        matches!(&dkg_manager.state, InnerState::InProgress { start_time_us, my_node, .. } if *start_time_us == 1700000000000000 && *my_node.metadata() == DKGNodeMetadata::new_for_test(999, addrs[0]))
    );

    // In state `InProgress`, DKGManager should respond to `DKGNodeRequest` with its own node.
    let rpc_node_request = new_rpc_node_request(999, addrs[3], rpc_response_collector.clone());
    let handle_result = dkg_manager.process_peer_rpc_msg(rpc_node_request).await;
    assert!(handle_result.is_ok());
    let last_responses = std::mem::replace(&mut *rpc_response_collector.write(), vec![])
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        vec![DKGMessage::NodeResponse(dkg_manager.state.my_node_cloned())],
        last_responses
    );
    assert!(matches!(&dkg_manager.state, InnerState::InProgress { .. }));

    // In state `InProgress`, DKGManager should accept `DKGAggNode`:
    // it should  update validator txn pool, and enter state `Finished`.
    let agg_node = DKGAggNode {
        metadata: DKGAggNodeMetadata::new(999, addrs[0]),
        agg_trx: DKGTranscriptWrapper::dummy(),
    };
    let handle_result = dkg_manager.process_agg_node(agg_node.clone()).await;
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
        vec![ValidatorTransaction::DKGTranscriptForNextEpoch(agg_node)],
        available_vtxns
    );
    assert!(matches!(&dkg_manager.state, InnerState::Finished { .. }));

    // In state `Finished`, DKGManager should still respond to `DKGNodeRequest` with its own node.
    let rpc_node_request = new_rpc_node_request(999, addrs[3], rpc_response_collector.clone());
    let handle_result = dkg_manager.process_peer_rpc_msg(rpc_node_request).await;
    assert!(handle_result.is_ok());
    let last_responses = std::mem::replace(&mut *rpc_response_collector.write(), vec![])
        .into_iter()
        .map(Result::unwrap)
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
    response_collector: Arc<RwLock<Vec<Result<DKGMessage>>>>,
) -> IncomingRpcRequest {
    IncomingRpcRequest {
        msg: DKGMessage::NodeRequest(DKGNodeRequest::new(epoch)),
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(response_collector)),
    }
}
