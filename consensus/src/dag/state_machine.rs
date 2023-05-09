use super::reliable_broadcast::ReliableBroadcastCommand;
use crate::{
    dag::{dag_driver::DagDriver, reliable_broadcast::ReliableBroadcast},
    network::{DagSender, NetworkSender},
    network_interface::ConsensusMsg,
    round_manager::VerifiedEvent,
    state_replication::PayloadClient,
};
use anyhow::Result;
use aptos_channels::aptos_channel;
use aptos_config::config::DagConfig;
use aptos_consensus_types::common::{Author, Payload, PayloadFilter};
use aptos_logger::info;
use aptos_types::PeerId;
use async_trait::async_trait;
use futures::StreamExt;
use futures_channel::oneshot;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Command {
    DagNodeProposal(Payload),
    ReliableBroadcastCommand(ReliableBroadcastCommand),
}

/// Input is the input to the state machine.
pub(crate) enum StateMachineEvent {
    VerifiedEvent(VerifiedEvent),
    Command(Command),
}

#[derive(Debug)]
pub(crate) struct OutgoingMessage {
    pub message: ConsensusMsg,
    pub maybe_recipients: Option<Vec<Author>>,
}

#[derive(Default, Debug)]
pub(crate) struct Actions {
    pub messages: Vec<OutgoingMessage>,
    pub command: Option<Command>,
    pub generate_proposal: Option<PayloadFilter>,
    // TODO: statesync data
}

/// StateMachine is the interface that a state machine needs to implement.
#[async_trait]
pub(crate) trait StateMachine {
    async fn tick(&mut self);
    async fn step(&mut self, input: StateMachineEvent) -> Result<()>;
    fn has_ready(&self) -> bool;
    async fn ready(&mut self) -> Option<Actions>;
}

pub struct StateMachineLoop {
    dag_driver: DagDriver,
    rb: ReliableBroadcast,

    dag_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    rb_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,

    config: DagConfig,
    payload_client: Arc<dyn PayloadClient>,
    network_sender: NetworkSender,
}

impl StateMachineLoop {
    pub fn new(
        dag_driver: DagDriver,
        rb: ReliableBroadcast,
        dag_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        rb_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        config: DagConfig,
        payload_client: Arc<dyn PayloadClient>,
        network_sender: NetworkSender,
    ) -> Self {
        Self {
            dag_driver,
            rb,

            dag_network_msg_rx,
            rb_network_msg_rx,

            config,
            payload_client,
            network_sender,
        }
    }

    async fn generate_proposal(&self, payload_filter: PayloadFilter) -> Payload {
        let payload = self
            .payload_client
            .pull_payload_for_dag(
                self.config.max_node_txns,
                self.config.max_node_bytes,
                payload_filter,
            )
            .await
            .expect("DAG: fail to retrieve payload");
        payload
    }

    pub async fn run(mut self, close_rx: oneshot::Receiver<oneshot::Sender<()>>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(10));

        loop {
            tokio::select! {
                biased;

                _ = interval.tick() => {
                    self.dag_driver.tick().await;
                    self.rb.tick().await;
                },

                Some(msg) = self.dag_network_msg_rx.next() => {
                    self.dag_driver.step(StateMachineEvent::VerifiedEvent(msg)).await.unwrap();
                },

                Some(msg) = self.rb_network_msg_rx.next() => {
                    self.rb.step(StateMachineEvent::VerifiedEvent(msg)).await.unwrap();
                },

                Some(actions) = self.dag_driver.ready() => {
                    // info!("DAG: ready {:?}", actions);
                    if let Some(payload_filter) = actions.generate_proposal {
                        let payload = self.generate_proposal(payload_filter).await;
                        self.dag_driver.step(StateMachineEvent::Command(Command::DagNodeProposal(payload))).await.unwrap();
                    }
                    for msg in actions.messages {
                        self.network_sender.send_consensus_msg(msg).await;
                    }
                    if let Some(cmd) = actions.command {
                        self.rb.step(StateMachineEvent::Command(cmd)).await.unwrap();
                    }
                },

                Some(actions) = self.rb.ready() => {
                    for msg in actions.messages {
                        self.network_sender.send_consensus_msg(msg).await;
                    }
                },
            }
        }
    }
}
