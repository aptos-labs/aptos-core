// Copyright © Aptos Foundation
use crate::{agg_trx_producer::TAggTranscriptProducer, network::IncomingRpcRequest, DKGMessage};
use anyhow::{anyhow, bail, ensure, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::Uniform;
use aptos_logger::error;
use aptos_types::{
    dkg::{
        DKGSessionMetadata, DKGSessionState, DKGStartEvent, DKGTrait, DKGTranscript,
        DKGTranscriptMetadata,
    },
    epoch_state::EpochState,
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use futures_channel::oneshot;
use futures_util::{future::AbortHandle, FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum InnerState<DKG: DKGTrait> {
    NotStarted,
    InProgress {
        start_time_us: u64,
        public_params: DKG::PublicParams,
        my_transcript: DKGTranscript,
        abort_handle: AbortHandle,
    },
    Finished {
        vtxn_guard: TxnGuard,
        start_time_us: u64,
        my_transcript: DKGTranscript,
        pull_confirmed: bool,
    },
}

impl<DKG: DKGTrait> InnerState<DKG> {
    fn variant_name(&self) -> &str {
        match self {
            InnerState::NotStarted => "NotStarted",
            InnerState::InProgress { .. } => "InProgress",
            InnerState::Finished { .. } => "Finished",
        }
    }

    #[cfg(test)]
    pub fn my_node_cloned(&self) -> DKGTranscript {
        match self {
            InnerState::NotStarted => panic!("my_node unavailable"),
            InnerState::InProgress {
                my_transcript: my_node,
                ..
            }
            | InnerState::Finished {
                my_transcript: my_node,
                ..
            } => my_node.clone(),
        }
    }
}

impl<DKG: DKGTrait> Default for InnerState<DKG> {
    fn default() -> Self {
        Self::NotStarted
    }
}

#[allow(dead_code)]
pub struct DKGManager<DKG: DKGTrait> {
    dealer_sk: Arc<DKG::DealerPrivateKey>,
    my_index: usize,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,

    vtxn_pool: VTxnPoolState,
    agg_trx_producer: Arc<dyn TAggTranscriptProducer<DKG>>,
    agg_trx_tx: Option<aptos_channel::Sender<(), DKG::Transcript>>,

    // When we put vtxn in the pool, we also put a copy of this so later pool can notify us.
    pull_notification_tx: aptos_channel::Sender<(), Arc<ValidatorTransaction>>,
    pull_notification_rx: aptos_channel::Receiver<(), Arc<ValidatorTransaction>>,

    // Control states.
    stopped: bool,
    state: InnerState<DKG>,
}

impl<DKG: DKGTrait> DKGManager<DKG> {
    pub fn new(
        dealer_sk: Arc<DKG::DealerPrivateKey>,
        my_index: usize,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        agg_trx_producer: Arc<dyn TAggTranscriptProducer<DKG>>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        let (pull_notification_tx, pull_notification_rx) =
            aptos_channel::new(QueueStyle::KLAST, 1, None);
        Self {
            dealer_sk,
            my_index,
            my_addr,
            epoch_state,
            vtxn_pool,
            agg_trx_tx: None,
            pull_notification_tx,
            pull_notification_rx,
            agg_trx_producer,
            stopped: false,
            state: InnerState::NotStarted,
        }
    }

    pub async fn run(
        mut self,
        in_progress_session: Option<DKGSessionState>,
        dkg_start_event_rx: oneshot::Receiver<DKGStartEvent>,
        mut rpc_msg_rx: aptos_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        if let Some(session_state) = in_progress_session {
            let DKGSessionState {
                metadata,
                start_time_us,
                ..
            } = session_state;
            self.setup_deal_broadcast(start_time_us, &metadata)
                .await
                .expect("setup_deal_broadcast() should be infallible");
        }

        let (agg_trx_tx, mut agg_trx_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.agg_trx_tx = Some(agg_trx_tx);

        let mut dkg_start_event_rx = dkg_start_event_rx.into_stream();
        let mut close_rx = close_rx.into_stream();
        while !self.stopped {
            let handling_result = tokio::select! {
                dkg_start_event = dkg_start_event_rx.select_next_some() => {
                    self.process_dkg_start_event(dkg_start_event.ok()).await
                },
                (_sender, msg) = rpc_msg_rx.select_next_some() => {
                    self.process_peer_rpc_msg(msg).await
                },
                agg_node = agg_trx_rx.select_next_some() => {
                    self.process_aggregated_transcript(agg_node).await
                },
                dkg_txn = self.pull_notification_rx.select_next_some() => {
                    self.process_dkg_txn_pulled_notification(dkg_txn).await
                },
                close_req = close_rx.select_next_some() => {
                    self.process_close_cmd(close_req.ok())
                }
            };

            if let Err(e) = handling_result {
                error!("{}", e);
            }
        }
    }

    /// On a CLOSE command from epoch manager, do clean-up.
    fn process_close_cmd(&mut self, ack_tx: Option<oneshot::Sender<()>>) -> Result<()> {
        self.stopped = true;

        if let InnerState::InProgress { abort_handle, .. } = &self.state {
            abort_handle.abort();
        }

        if let Some(tx) = ack_tx {
            let _ = tx.send(());
        }

        Ok(())
    }

    /// On a vtxn pull notification, record metric.
    async fn process_dkg_txn_pulled_notification(
        &mut self,
        _txn: Arc<ValidatorTransaction>,
    ) -> Result<()> {
        if let InnerState::Finished { pull_confirmed, .. } = &mut self.state {
            if !*pull_confirmed {
                // TODO(zjma): metric DKG_AGG_NODE_PROPOSED
            }
            *pull_confirmed = true;
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
        dkg_session_metadata: &DKGSessionMetadata,
    ) -> Result<()> {
        self.state = match &self.state {
            InnerState::NotStarted => {
                let public_params = DKG::new_public_params(dkg_session_metadata);
                let mut rng = thread_rng();
                let input_secret = if cfg!(feature = "smoke-test") {
                    DKG::generate_predictable_input_secret_for_testing(self.dealer_sk.as_ref())
                } else {
                    DKG::InputSecret::generate(&mut rng)
                };

                let trx = DKG::generate_transcript(
                    &mut rng,
                    &public_params,
                    &input_secret,
                    self.my_index as u64,
                    &self.dealer_sk,
                );

                let dkg_transcript = DKGTranscript::new(
                    self.epoch_state.epoch,
                    self.my_addr,
                    bcs::to_bytes(&trx).map_err(|e| {
                        anyhow!("setup_deal_broadcast failed with trx serialization error: {e}")
                    })?,
                );

                // TODO(zjma): DKG_NODE_READY metric

                let abort_handle = self.agg_trx_producer.start_produce(
                    self.epoch_state.clone(),
                    public_params.clone(),
                    self.agg_trx_tx.clone(),
                );

                // Switch to the next stage.
                InnerState::InProgress {
                    start_time_us,
                    public_params,
                    my_transcript: dkg_transcript,
                    abort_handle,
                }
            },
            _ => unreachable!(), // `setup_deal_broadcast` is called only when DKG state is `NotStarted`.
        };

        Ok(())
    }

    /// On a locally aggregated transcript, put it into the validator txn pool and update inner states.
    async fn process_aggregated_transcript(&mut self, agg_trx: DKG::Transcript) -> Result<()> {
        self.state = match std::mem::take(&mut self.state) {
            InnerState::InProgress {
                start_time_us,
                my_transcript: my_node,
                ..
            } => {
                // TODO(zjma): metric DKG_AGG_NODE_READY
                let txn = ValidatorTransaction::DKGResult(DKGTranscript {
                    metadata: DKGTranscriptMetadata {
                        epoch: self.epoch_state.epoch,
                        author: self.my_addr,
                    },
                    transcript_bytes: bcs::to_bytes(&agg_trx).map_err(|e|anyhow!("process_aggregated_transcript failed with trx serialization error: {e}"))?,
                });
                let vtxn_guard = self.vtxn_pool.put(
                    Topic::DKG,
                    Arc::new(txn),
                    Some(self.pull_notification_tx.clone()),
                );
                InnerState::Finished {
                    vtxn_guard,
                    start_time_us,
                    my_transcript: my_node,
                    pull_confirmed: false,
                }
            },
            _ => bail!("process agg trx failed with invalid inner state"),
        };
        Ok(())
    }

    /// On a DKG start event, execute DKG.
    async fn process_dkg_start_event(&mut self, maybe_event: Option<DKGStartEvent>) -> Result<()> {
        if let Some(event) = maybe_event {
            let DKGStartEvent {
                session_metadata,
                start_time_us,
            } = event;
            ensure!(self.epoch_state.epoch == session_metadata.dealer_epoch);
            self.setup_deal_broadcast(start_time_us, &session_metadata)
                .await?;
        }
        Ok(())
    }

    /// Process an RPC request from DKG peers.
    async fn process_peer_rpc_msg(&mut self, req: IncomingRpcRequest) -> Result<()> {
        let IncomingRpcRequest {
            msg,
            mut response_sender,
            ..
        } = req;
        ensure!(msg.epoch() == self.epoch_state.epoch);
        let response = match (&self.state, &msg) {
            (
                InnerState::Finished {
                    my_transcript: my_node,
                    ..
                },
                DKGMessage::NodeRequest(_),
            )
            | (
                InnerState::InProgress {
                    my_transcript: my_node,
                    ..
                },
                DKGMessage::NodeRequest(_),
            ) => Ok(DKGMessage::NodeResponse(my_node.clone())),
            _ => Err(anyhow!(
                "msg {:?} unexpected in state {:?}",
                msg.name(),
                self.state.variant_name()
            )),
        };

        response_sender.send(response);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
