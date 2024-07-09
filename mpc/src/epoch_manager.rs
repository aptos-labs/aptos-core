// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::time::Duration;
use futures_channel::oneshot;
use futures_util::StreamExt;
use tokio_retry::strategy::ExponentialBackoff;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel;
use aptos_channels::message_queues::QueueStyle;
use aptos_config::config::ReliableBroadcastConfig;
use aptos_event_notifications::{EventNotification, EventNotificationListener, ReconfigNotification, ReconfigNotificationListener};
use aptos_network::application::interface::NetworkClient;
use aptos_network::protocols::network::Event;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::epoch_state::EpochState;
use aptos_types::mpc::{MPCEventMoveStruct, MPCState};
use aptos_types::on_chain_config::{OnChainConfigPayload, OnChainConfigProvider, ValidatorSet};
use aptos_validator_transaction_pool::VTxnPoolState;
use move_core_types::account_address::AccountAddress;
use crate::mpc_manager::MPCManager;
use crate::network::{IncomingRpcRequest, NetworkReceivers, NetworkSender};
use crate::types::MPCMessage;
use anyhow::Result;
use aptos_logger::{debug, error};
use crate::network_interface::MPCNetworkClient;

pub struct EpochManager<P: OnChainConfigProvider> {
    // Some useful metadata
    my_addr: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,

    // Inbound events
    reconfig_events: ReconfigNotificationListener<P>,
    mpc_events: EventNotificationListener,

    // Msgs to MPC manager
    mpc_rpc_msg_tx: Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
    mpc_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    mpc_event_tx: Option<aptos_channel::Sender<(), MPCEventMoveStruct>>,
    vtxn_pool: VTxnPoolState,

    // Network utils
    self_sender: aptos_channels::Sender<Event<MPCMessage>>,
    network_sender: MPCNetworkClient<NetworkClient<MPCMessage>>,
    rb_config: ReliableBroadcastConfig,

}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        my_addr: AccountAddress,
        reconfig_events: ReconfigNotificationListener<P>,
        mpc_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<MPCMessage>>,
        network_sender: MPCNetworkClient<NetworkClient<MPCMessage>>,
        vtxn_pool: VTxnPoolState,
        rb_config: ReliableBroadcastConfig,
    ) -> Self {
        Self {
            my_addr,
            epoch_state: None,
            reconfig_events,
            mpc_events,
            mpc_rpc_msg_tx: None,
            mpc_manager_close_tx: None,
            self_sender,
            network_sender,
            vtxn_pool,
            mpc_event_tx: None,
            rb_config,
        }
    }

    fn on_mpc_event_notification(&mut self, notification: EventNotification) -> Result<()> {
        debug!(
            epoch = self.epoch_state.as_ref().unwrap().epoch,
            "0722 - on_mpc_event_notification: start"
        );
        if let Some(tx) = self.mpc_event_tx.as_ref() {
            debug!(
                epoch = self.epoch_state.as_ref().unwrap().epoch,
                "0722 - on_mpc_event_notification: has tx to MPCManager"
            );
            let EventNotification {
                subscribed_events, ..
            } = notification;
            for event in subscribed_events {
                debug!(
                    epoch = self.epoch_state.as_ref().unwrap().epoch,
                    "0722 - on_mpc_event_notification: notification contains 1+ events"
                );
                match MPCEventMoveStruct::try_from(&event) {
                    Ok(mpc_event) => {
                        debug!(
                            epoch = self.epoch_state.as_ref().unwrap().epoch,
                            "0722 - on_mpc_event_notification: cast succeeded"
                        );
                        let _ = tx.push((), mpc_event);
                        return Ok(());
                    },
                    Err(e) => {
                        error!("[MPC] on_mpc_event_notification: failed with event conversion error: {e}");
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn start(mut self, mut network_receivers: NetworkReceivers) {
        self.await_reconfig_notification().await;
        loop {
            let handling_result = tokio::select! {
                notification = self.mpc_events.select_next_some() => {
                    self.on_mpc_event_notification(notification)
                },
                reconfig_notification = self.reconfig_events.select_next_some() => {
                    self.on_new_epoch(reconfig_notification).await
                },
            };
        }
    }
    async fn await_reconfig_notification(&mut self) {
        let reconfig_notification = self
            .reconfig_events
            .next()
            .await
            .expect("Reconfig sender dropped, unable to start new epoch");
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await;
    }

    async fn start_new_epoch(&mut self, payload: OnChainConfigPayload<P>) {
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");

        let epoch_state = Arc::new(EpochState {
            epoch: payload.epoch(),
            verifier: (&validator_set).into(),
        });
        self.epoch_state = Some(epoch_state.clone());
        let my_index = epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.my_addr)
            .copied();

        let mpc_enabled = true; //mpc todo: check on-chain state instead
        if let (true, Some(my_index)) = (mpc_enabled, my_index) {
            let mpc_state = payload.get::<MPCState>().unwrap_or_default();

            let network_sender = self.create_network_sender();
            let rb = ReliableBroadcast::new(
                self.my_addr,
                epoch_state.verifier.get_ordered_account_addresses(),
                Arc::new(network_sender),
                ExponentialBackoff::from_millis(self.rb_config.backoff_policy_base_ms)
                    .factor(self.rb_config.backoff_policy_factor)
                    .max_delay(Duration::from_millis(
                        self.rb_config.backoff_policy_max_delay_ms,
                    )),
                aptos_time_service::TimeService::real(),
                Duration::from_millis(self.rb_config.rpc_timeout_ms),
                BoundedExecutor::new(8, tokio::runtime::Handle::current()),
            );

            let (mpc_event_tx, mpc_event_rx) =
                aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.mpc_event_tx = Some(mpc_event_tx);

            let (mpc_rpc_msg_tx, mpc_rpc_msg_rx) = aptos_channel::new::<
                AccountAddress,
                (AccountAddress, IncomingRpcRequest),
            >(QueueStyle::FIFO, 100, None);
            self.mpc_rpc_msg_tx = Some(mpc_rpc_msg_tx);
            let (mpc_manager_close_tx, mpc_manager_close_rx) = oneshot::channel();
            self.mpc_manager_close_tx = Some(mpc_manager_close_tx);

            let mpc_manager = MPCManager::new(my_index, self.my_addr, epoch_state, self.vtxn_pool.clone());
            tokio::spawn(mpc_manager.run(
                mpc_state,
                mpc_event_rx,
                mpc_rpc_msg_rx,
                mpc_manager_close_rx,
            ));
        }
    }

    async fn on_new_epoch(&mut self, reconfig_notification: ReconfigNotification<P>) -> Result<()> {
        self.shutdown_current_processor().await;
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await;
        Ok(())
    }

    async fn shutdown_current_processor(&mut self) {
        if let Some(tx) = self.mpc_manager_close_tx.take() {
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
