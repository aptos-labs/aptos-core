// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    agg_trx_producer::AggTranscriptProducer,
    chunky::dkg_manager::ChunkyDKGManager,
    dkg_manager::DKGManager,
    network::{IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::DKGNetworkClient,
    DKGMessage,
};
use anyhow::{anyhow, Result};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{ReliableBroadcastConfig, SafetyRulesConfig};
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_logger::{error, info, warn};
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_safety_rules::{safety_rules_manager::storage, PersistentSafetyStorage};
use aptos_types::{
    account_address::AccountAddress,
    dkg::{
        chunky_dkg::{ChunkyDKGStartEvent, ChunkyDKGState},
        DKGStartEvent, DKGState, DefaultDKG,
    },
    epoch_state::EpochState,
    on_chain_config::{
        ChunkyDKGConfigMoveStruct, OnChainChunkyDKGConfig, OnChainConfigPayload,
        OnChainConfigProvider, OnChainConsensusConfig, OnChainRandomnessConfig,
        RandomnessConfigMoveStruct, RandomnessConfigSeqNum, ValidatorSet,
    },
};
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::StreamExt;
use futures_channel::oneshot;
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

const DKG_START_EVENT_TYPE_TAG: &str = "0x1::dkg::DKGStartEvent";
const CHUNKY_DKG_START_EVENT_TYPE_TAG: &str = "0x1::chunky_dkg::ChunkyDKGStartEvent";

pub struct EpochManager<P: OnChainConfigProvider> {
    // Some useful metadata
    my_addr: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,

    // Inbound events
    reconfig_events: ReconfigNotificationListener<P>,
    dkg_start_events: EventNotificationListener,

    // Msgs to DKG manager
    dkg_rpc_msg_tx:
        Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
    dkg_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    dkg_start_event_tx: Option<aptos_channel::Sender<(), DKGStartEvent>>,

    vtxn_pool: VTxnPoolState,

    chunky_dkg_rpc_msg_tx:
        Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
    chunky_dkg_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    chunky_dkg_start_event_tx: Option<aptos_channel::Sender<(), ChunkyDKGStartEvent>>,

    // Network utils
    self_sender: aptos_channels::Sender<Event<DKGMessage>>,
    network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
    rb_config: ReliableBroadcastConfig,

    // Randomness overriding.
    randomness_override_seq_num: u64,

    key_storage: PersistentSafetyStorage,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        safety_rules_config: &SafetyRulesConfig,
        my_addr: AccountAddress,
        reconfig_events: ReconfigNotificationListener<P>,
        dkg_start_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<DKGMessage>>,
        network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
        vtxn_pool: VTxnPoolState,
        rb_config: ReliableBroadcastConfig,
        randomness_override_seq_num: u64,
    ) -> Self {
        Self {
            my_addr,
            epoch_state: None,
            reconfig_events,
            dkg_start_events,
            dkg_rpc_msg_tx: None,
            dkg_manager_close_tx: None,
            dkg_start_event_tx: None,
            self_sender,
            network_sender,
            vtxn_pool,
            chunky_dkg_manager_close_tx: None,
            chunky_dkg_rpc_msg_tx: None,
            chunky_dkg_start_event_tx: None,
            rb_config,
            randomness_override_seq_num,
            key_storage: storage(safety_rules_config),
        }
    }

    fn process_rpc_request(
        &mut self,
        peer_id: AccountAddress,
        dkg_request: IncomingRpcRequest,
    ) -> Result<()> {
        if Some(dkg_request.msg.epoch()) != self.epoch_state.as_ref().map(|s| s.epoch) {
            return Ok(());
        }

        // Check if this is a Chunky message and forward to the appropriate channel
        let is_chunky_message = matches!(
            &dkg_request.msg,
            DKGMessage::ChunkyTranscriptRequest(_)
                | DKGMessage::ChunkyTranscriptResponse(_)
                | DKGMessage::SubtranscriptSignatureRequest(_)
                | DKGMessage::SubtranscriptSignatureResponse(_)
                | DKGMessage::MissingTranscriptRequest(_)
                | DKGMessage::MissingTranscriptResponse(_),
        );

        if is_chunky_message {
            // Forward to ChunkyDKGManager if it is alive.
            if let Some(tx) = &self.chunky_dkg_rpc_msg_tx {
                let _ = tx.push(peer_id, (peer_id, dkg_request));
            }
        } else {
            // Forward to DKGManager if it is alive.
            if let Some(tx) = &self.dkg_rpc_msg_tx {
                let _ = tx.push(peer_id, (peer_id, dkg_request));
            }
        }
        Ok(())
    }

    fn on_dkg_start_notification(&mut self, notification: EventNotification) -> Result<()> {
        let EventNotification {
            subscribed_events, ..
        } = notification;
        for event in subscribed_events {
            let type_tag_str = event.type_tag().to_canonical_string();
            match type_tag_str.as_str() {
                DKG_START_EVENT_TYPE_TAG => {
                    if let Some(tx) = self.dkg_start_event_tx.as_ref() {
                        if let Ok(dkg_start_event) = DKGStartEvent::try_from(&event) {
                            let _ = tx.push((), dkg_start_event);
                            return Ok(());
                        } else {
                            error!("[DKG] on_dkg_start_notification: failed in converting a contract event to a DKGStartEvent!");
                        }
                    }
                },
                CHUNKY_DKG_START_EVENT_TYPE_TAG => {
                    // TODO(ibalajiarun): Use unbounded sender for tx for simplicity
                    if let Some(tx) = self.chunky_dkg_start_event_tx.as_ref() {
                        match ChunkyDKGStartEvent::try_from(&event) {
                            Ok(dkg_start_event) => {
                                let _ = tx.push((), dkg_start_event);
                                return Ok(());
                            },
                            Err(e) => {
                                error!("[DKG] Failed conversion to ChunkyDKGStartEvent: {}", e);
                            },
                        }
                    }
                },
                unknown_type_tag => error!(
                    "[DKG] on_dkg_start_notification for unknown type tag: {}",
                    unknown_type_tag
                ),
            }
        }
        Ok(())
    }

    pub async fn start(mut self, mut network_receivers: NetworkReceivers) {
        self.await_reconfig_notification().await;
        loop {
            let handling_result = tokio::select! {
                notification = self.dkg_start_events.select_next_some() => {
                    self.on_dkg_start_notification(notification)
                },
                reconfig_notification = self.reconfig_events.select_next_some() => {
                    self.on_new_epoch(reconfig_notification).await
                },
                (peer, rpc_request) = network_receivers.rpc_rx.select_next_some() => {
                    self.process_rpc_request(peer, rpc_request)
                },
            };

            if let Err(e) = handling_result {
                error!("{}", e);
            }
        }
    }

    async fn await_reconfig_notification(&mut self) {
        let reconfig_notification = self
            .reconfig_events
            .next()
            .await
            .expect("Reconfig sender dropped, unable to start new epoch");
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await
            .unwrap();
    }

    async fn start_new_epoch(&mut self, payload: OnChainConfigPayload<P>) -> Result<()> {
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");

        let epoch_state = Arc::new(EpochState::new(payload.epoch(), (&validator_set).into()));
        self.epoch_state = Some(epoch_state.clone());
        let Some(my_index) = epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.my_addr)
            .copied()
        else {
            warn!(
                "This validator is not in current epoch {}",
                epoch_state.epoch
            );
            return Ok(());
        };

        let onchain_randomness_config_seq_num = payload
            .get::<RandomnessConfigSeqNum>()
            .unwrap_or_else(|_| RandomnessConfigSeqNum::default_if_missing());

        let randomness_config_move_struct = payload.get::<RandomnessConfigMoveStruct>();

        info!(
            epoch = epoch_state.epoch,
            local = self.randomness_override_seq_num,
            onchain = onchain_randomness_config_seq_num.seq_num,
            "Checking randomness config override."
        );
        if self.randomness_override_seq_num > onchain_randomness_config_seq_num.seq_num {
            warn!("Randomness will be force-disabled by local config!");
        }

        let onchain_randomness_config = OnChainRandomnessConfig::from_configs(
            self.randomness_override_seq_num,
            onchain_randomness_config_seq_num.seq_num,
            randomness_config_move_struct.ok(),
        );

        let chunky_dkg_config_move_struct = payload.get::<ChunkyDKGConfigMoveStruct>();

        let onchain_chunky_dkg_config =
            OnChainChunkyDKGConfig::from_configs(chunky_dkg_config_move_struct.ok());

        let onchain_consensus_config = payload
            .get::<OnChainConsensusConfig>()
            .inspect_err(|e| {
                error!("Failed to read on-chain consensus config {}", e);
            })
            .unwrap_or_default();

        // Create shared network sender and reliable broadcast for both DKG managers
        let network_sender = Arc::new(self.create_network_sender());
        let rb = Arc::new(ReliableBroadcast::new(
            self.my_addr,
            epoch_state.verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            ExponentialBackoff::from_millis(self.rb_config.backoff_policy_base_ms)
                .factor(self.rb_config.backoff_policy_factor)
                .max_delay(Duration::from_millis(
                    self.rb_config.backoff_policy_max_delay_ms,
                )),
            aptos_time_service::TimeService::real(),
            Duration::from_millis(self.rb_config.rpc_timeout_ms),
            BoundedExecutor::new(8, tokio::runtime::Handle::current()),
        ));

        // Check both validator txn and randomness features are enabled
        if onchain_consensus_config.is_vtxn_enabled()
            && onchain_randomness_config.randomness_enabled()
        {
            self.start_dkg_manager(epoch_state.clone(), my_index, &payload, rb.clone())
                .await?;
        }

        // Check both validator txn and chunky dkg features are enabled
        if onchain_consensus_config.is_vtxn_enabled()
            && onchain_chunky_dkg_config.chunky_dkg_enabled()
        {
            self.start_chunky_dkg_manager(
                epoch_state.clone(),
                my_index,
                &payload,
                rb.clone(),
                network_sender,
            )
            .await?;
        }
        Ok(())
    }

    async fn start_dkg_manager(
        &mut self,
        epoch_state: Arc<EpochState>,
        my_index: usize,
        payload: &OnChainConfigPayload<P>,
        rb: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    ) -> Result<()> {
        let DKGState {
            in_progress: in_progress_session,
            ..
        } = payload.get::<DKGState>().unwrap_or_default();

        let agg_trx_producer = AggTranscriptProducer::new(rb);

        let (dkg_start_event_tx, dkg_start_event_rx) =
            aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.dkg_start_event_tx = Some(dkg_start_event_tx);

        let (dkg_rpc_msg_tx, dkg_rpc_msg_rx) = aptos_channel::new::<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >(QueueStyle::FIFO, 100, None);
        self.dkg_rpc_msg_tx = Some(dkg_rpc_msg_tx);
        let (dkg_manager_close_tx, dkg_manager_close_rx) = oneshot::channel();
        self.dkg_manager_close_tx = Some(dkg_manager_close_tx);
        let my_pk = epoch_state
            .verifier
            .get_public_key(&self.my_addr)
            .ok_or_else(|| anyhow!("my pk not found in validator set"))?;
        let dealer_sk = self
            .key_storage
            .consensus_sk_by_pk(my_pk.clone())
            .map_err(|e| {
                anyhow!("dkg new epoch handling failed with consensus sk lookup err: {e}")
            })?;
        let dkg_manager = DKGManager::<DefaultDKG>::new(
            Arc::new(dealer_sk),
            Arc::new(my_pk),
            my_index,
            self.my_addr,
            epoch_state,
            Arc::new(agg_trx_producer),
            self.vtxn_pool.clone(),
        );
        tokio::spawn(dkg_manager.run(
            in_progress_session,
            dkg_start_event_rx,
            dkg_rpc_msg_rx,
            dkg_manager_close_rx,
        ));
        Ok(())
    }

    async fn start_chunky_dkg_manager(
        &mut self,
        epoch_state: Arc<EpochState>,
        my_index: usize,
        payload: &OnChainConfigPayload<P>,
        rb: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
        network_sender: Arc<NetworkSender>,
    ) -> Result<()> {
        let ChunkyDKGState {
            in_progress: in_progress_session,
            ..
        } = payload.get::<ChunkyDKGState>().unwrap_or_default();

        let (chunky_dkg_start_event_tx, chunky_dkg_start_event_rx) =
            aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.chunky_dkg_start_event_tx = Some(chunky_dkg_start_event_tx);

        let (chunky_dkg_rpc_msg_tx, chunky_dkg_rpc_msg_rx) = aptos_channel::new::<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >(QueueStyle::FIFO, 100, None);
        self.chunky_dkg_rpc_msg_tx = Some(chunky_dkg_rpc_msg_tx);
        let (chunky_dkg_manager_close_tx, chunky_dkg_manager_close_rx) = oneshot::channel();
        self.chunky_dkg_manager_close_tx = Some(chunky_dkg_manager_close_tx);
        let my_pk = epoch_state
            .verifier
            .get_public_key(&self.my_addr)
            .ok_or_else(|| anyhow!("my pk not found in validator set"))?;
        let dealer_sk = self
            .key_storage
            .consensus_sk_by_pk(my_pk.clone())
            .map_err(|e| {
                anyhow!("chunky dkg new epoch handling failed with consensus sk lookup err: {e}")
            })?;
        let chunky_dkg_manager = ChunkyDKGManager::new(
            Arc::new(dealer_sk),
            Arc::new(my_pk),
            my_index,
            self.my_addr,
            epoch_state,
            self.vtxn_pool.clone(),
            rb,
            network_sender,
        );
        tokio::spawn(chunky_dkg_manager.run(
            in_progress_session,
            chunky_dkg_start_event_rx,
            chunky_dkg_rpc_msg_rx,
            chunky_dkg_manager_close_rx,
        ));
        Ok(())
    }

    async fn on_new_epoch(&mut self, reconfig_notification: ReconfigNotification<P>) -> Result<()> {
        self.shutdown_current_processor().await;
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await?;
        Ok(())
    }

    async fn shutdown_current_processor(&mut self) {
        if let Some(tx) = self.dkg_manager_close_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ack_tx).unwrap();
            ack_rx.await.unwrap();
        }
        if let Some(tx) = self.chunky_dkg_manager_close_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ack_tx).unwrap();
            ack_rx.await.unwrap();
        }
    }

    fn create_network_sender(&self) -> NetworkSender {
        NetworkSender::new(
            self.my_addr,
            self.network_sender.clone(),
            self.self_sender.clone(),
        )
    }
}
