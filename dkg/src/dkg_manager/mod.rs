// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    agg_trx_producer::TAggTranscriptProducer,
    counters::{DKG_STAGE_SECONDS, ROUNDING_SECONDS},
    network::IncomingRpcRequest,
    DKGMessage,
};
use anyhow::{anyhow, bail, ensure, Result};
use velor_channels::{velor_channel, message_queues::QueueStyle};
use velor_crypto::Uniform;
use velor_infallible::duration_since_epoch;
use velor_logger::{debug, error, info, warn};
use velor_types::{
    dkg::{
        DKGSessionMetadata, DKGSessionState, DKGStartEvent, DKGTrait, DKGTranscript,
        DKGTranscriptMetadata, MayHaveRoundingSummary,
    },
    epoch_state::EpochState,
    validator_txn::{Topic, ValidatorTransaction},
};
use velor_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use fail::fail_point;
use futures_channel::oneshot;
use futures_util::{future::AbortHandle, FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use rand::{prelude::StdRng, thread_rng, SeedableRng};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug)]
enum InnerState {
    NotStarted,
    InProgress {
        start_time: Duration,
        my_transcript: DKGTranscript,
        abort_handle: AbortHandle,
    },
    Finished {
        vtxn_guard: TxnGuard,
        start_time: Duration,
        my_transcript: DKGTranscript,
        proposed: bool,
    },
}

impl Default for InnerState {
    fn default() -> Self {
        Self::NotStarted
    }
}

pub struct DKGManager<DKG: DKGTrait> {
    dealer_sk: Arc<DKG::DealerPrivateKey>,
    my_index: usize,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,

    vtxn_pool: VTxnPoolState,
    agg_trx_producer: Arc<dyn TAggTranscriptProducer<DKG>>,
    agg_trx_tx: Option<velor_channel::Sender<(), DKG::Transcript>>,

    // When we put vtxn in the pool, we also put a copy of this so later pool can notify us.
    pull_notification_tx: velor_channel::Sender<(), Arc<ValidatorTransaction>>,
    pull_notification_rx: velor_channel::Receiver<(), Arc<ValidatorTransaction>>,

    // Control states.
    stopped: bool,
    state: InnerState,
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
    pub fn my_node_cloned(&self) -> DKGTranscript {
        match self {
            InnerState::NotStarted => panic!("my_node unavailable"),
            InnerState::InProgress { my_transcript, .. }
            | InnerState::Finished { my_transcript, .. } => my_transcript.clone(),
        }
    }
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
            velor_channel::new(QueueStyle::KLAST, 1, None);
        Self {
            dealer_sk,
            my_addr,
            my_index,
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
        mut dkg_start_event_rx: velor_channel::Receiver<(), DKGStartEvent>,
        mut rpc_msg_rx: velor_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[DKG] DKGManager started."
        );
        let mut interval = tokio::time::interval(Duration::from_millis(5000));

        let (agg_trx_tx, mut agg_trx_rx) = velor_channel::new(QueueStyle::KLAST, 1, None);
        self.agg_trx_tx = Some(agg_trx_tx);

        if let Some(session_state) = in_progress_session {
            let DKGSessionState {
                start_time_us,
                metadata,
                ..
            } = session_state;

            if metadata.dealer_epoch == self.epoch_state.epoch {
                info!(
                    epoch = self.epoch_state.epoch,
                    "Found unfinished and current DKG session. Continuing it."
                );
                if let Err(e) = self.setup_deal_broadcast(start_time_us, &metadata).await {
                    error!(epoch = self.epoch_state.epoch, "dkg resumption failed: {e}");
                }
            } else {
                info!(
                    cur_epoch = self.epoch_state.epoch,
                    dealer_epoch = metadata.dealer_epoch,
                    "Found unfinished but stale DKG session. Ignoring it."
                );
            }
        }

        let mut close_rx = close_rx.into_stream();
        while !self.stopped {
            let handling_result = tokio::select! {
                dkg_start_event = dkg_start_event_rx.select_next_some() => {
                    self.process_dkg_start_event(dkg_start_event)
                        .await
                        .map_err(|e|anyhow!("[DKG] process_dkg_start_event failed: {e}"))
                },
                (_sender, msg) = rpc_msg_rx.select_next_some() => {
                    self.process_peer_rpc_msg(msg)
                        .await
                        .map_err(|e|anyhow!("[DKG] process_peer_rpc_msg failed: {e}"))
                },
                agg_transcript = agg_trx_rx.select_next_some() => {
                    self.process_aggregated_transcript(agg_transcript)
                        .await
                        .map_err(|e|anyhow!("[DKG] process_aggregated_transcript failed: {e}"))

                },
                dkg_txn = self.pull_notification_rx.select_next_some() => {
                    self.process_dkg_txn_pulled_notification(dkg_txn)
                        .await
                        .map_err(|e|anyhow!("[DKG] process_dkg_txn_pulled_notification failed: {e}"))
                },
                close_req = close_rx.select_next_some() => {
                    self.process_close_cmd(close_req.ok())
                },
                _ = interval.tick().fuse() => {
                    self.observe()
                },
            };

            if let Err(e) = handling_result {
                error!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr.to_hex().as_str(),
                    "[DKG] DKGManager handling error: {e}"
                );
            }
        }
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[DKG] DKGManager finished."
        );
    }

    fn observe(&self) -> Result<()> {
        debug!("[DKG] dkg_manager_state={:?}", self.state);
        Ok(())
    }

    /// On a CLOSE command from epoch manager, do clean-up.
    fn process_close_cmd(&mut self, ack_tx: Option<oneshot::Sender<()>>) -> Result<()> {
        self.stopped = true;

        match std::mem::take(&mut self.state) {
            InnerState::NotStarted => {},
            InnerState::InProgress { abort_handle, .. } => {
                abort_handle.abort();
            },
            InnerState::Finished {
                vtxn_guard,
                start_time,
                ..
            } => {
                let epoch_change_time = duration_since_epoch();
                let secs_since_dkg_start =
                    epoch_change_time.as_secs_f64() - start_time.as_secs_f64();
                DKG_STAGE_SECONDS
                    .with_label_values(&[self.my_addr.to_hex().as_str(), "epoch_change"])
                    .observe(secs_since_dkg_start);
                info!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr,
                    secs_since_dkg_start = secs_since_dkg_start,
                    "[DKG] txn executed and entering new epoch.",
                );

                drop(vtxn_guard);
            },
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
        match &mut self.state {
            InnerState::Finished {
                start_time,
                proposed,
                ..
            } => {
                if !*proposed {
                    *proposed = true;
                    let proposed_time = duration_since_epoch();
                    let secs_since_dkg_start =
                        proposed_time.as_secs_f64() - start_time.as_secs_f64();
                    DKG_STAGE_SECONDS
                        .with_label_values(&[self.my_addr.to_hex().as_str(), "proposed"])
                        .observe(secs_since_dkg_start);
                    info!(
                        epoch = self.epoch_state.epoch,
                        my_addr = self.my_addr,
                        secs_since_dkg_start = secs_since_dkg_start,
                        "[DKG] aggregated transcript proposed by consensus.",
                    );
                }
                Ok(())
            },
            _ => {
                bail!("[DKG] pull notification only expected in finished state");
            },
        }
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
        ensure!(
            matches!(&self.state, InnerState::NotStarted),
            "transcript already dealt"
        );
        let dkg_start_time = Duration::from_micros(start_time_us);
        let deal_start = duration_since_epoch();
        let secs_since_dkg_start = deal_start.as_secs_f64() - dkg_start_time.as_secs_f64();
        DKG_STAGE_SECONDS
            .with_label_values(&[self.my_addr.to_hex().as_str(), "deal_start"])
            .observe(secs_since_dkg_start);
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            secs_since_dkg_start = secs_since_dkg_start,
            "[DKG] Deal transcript started.",
        );
        let public_params = DKG::new_public_params(dkg_session_metadata);
        if let Some(summary) = public_params.rounding_summary() {
            info!(
                epoch = self.epoch_state.epoch,
                "Rounding summary: {:?}", summary
            );
            ROUNDING_SECONDS
                .with_label_values(&[summary.method.as_str()])
                .observe(summary.exec_time.as_secs_f64());
        }

        let mut rng = if cfg!(feature = "smoke-test") {
            StdRng::from_seed(self.my_addr.into_bytes())
        } else {
            StdRng::from_rng(thread_rng()).unwrap()
        };
        let input_secret = DKG::InputSecret::generate(&mut rng);

        let trx = DKG::generate_transcript(
            &mut rng,
            &public_params,
            &input_secret,
            self.my_index as u64,
            &self.dealer_sk,
        );

        let my_transcript = DKGTranscript::new(
            self.epoch_state.epoch,
            self.my_addr,
            bcs::to_bytes(&trx).map_err(|e| anyhow!("transcript serialization error: {e}"))?,
        );

        let deal_finish = duration_since_epoch();
        let secs_since_dkg_start = deal_finish.as_secs_f64() - dkg_start_time.as_secs_f64();
        DKG_STAGE_SECONDS
            .with_label_values(&[self.my_addr.to_hex().as_str(), "deal_finish"])
            .observe(secs_since_dkg_start);
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            secs_since_dkg_start = secs_since_dkg_start,
            "[DKG] Deal transcript finished.",
        );

        let abort_handle = self.agg_trx_producer.start_produce(
            dkg_start_time,
            self.my_addr,
            self.epoch_state.clone(),
            public_params.clone(),
            self.agg_trx_tx.clone(),
        );

        // Switch to the next stage.
        self.state = InnerState::InProgress {
            start_time: dkg_start_time,
            my_transcript,
            abort_handle,
        };

        Ok(())
    }

    /// On a locally aggregated transcript, put it into the validator txn pool and update inner states.
    async fn process_aggregated_transcript(&mut self, agg_trx: DKG::Transcript) -> Result<()> {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[DKG] Processing locally aggregated transcript."
        );
        self.state = match std::mem::take(&mut self.state) {
            InnerState::InProgress {
                start_time,
                my_transcript,
                ..
            } => {
                let agg_transcript_ready_time = duration_since_epoch();
                let secs_since_dkg_start =
                    agg_transcript_ready_time.as_secs_f64() - start_time.as_secs_f64();
                DKG_STAGE_SECONDS
                    .with_label_values(&[self.my_addr.to_hex().as_str(), "agg_transcript_ready"])
                    .observe(secs_since_dkg_start);

                let txn = ValidatorTransaction::DKGResult(DKGTranscript {
                    metadata: DKGTranscriptMetadata {
                        epoch: self.epoch_state.epoch,
                        author: self.my_addr,
                    },
                    transcript_bytes: bcs::to_bytes(&agg_trx)
                        .map_err(|e| anyhow!("transcript serialization error: {e}"))?,
                });
                let vtxn_guard = self.vtxn_pool.put(
                    Topic::DKG,
                    Arc::new(txn),
                    Some(self.pull_notification_tx.clone()),
                );
                info!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr,
                    "[DKG] aggregated transcript put into vtxn pool."
                );
                InnerState::Finished {
                    vtxn_guard,
                    start_time,
                    my_transcript,
                    proposed: false,
                }
            },
            _ => bail!("[DKG] aggregated transcript only expected during DKG"),
        };
        Ok(())
    }

    async fn process_dkg_start_event(&mut self, event: DKGStartEvent) -> Result<()> {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[DKG] Processing DKGStart event."
        );
        fail_point!("dkg::process_dkg_start_event");
        let DKGStartEvent {
            session_metadata,
            start_time_us,
        } = event;
        ensure!(
            matches!(&self.state, InnerState::NotStarted),
            "[DKG] dkg already started"
        );
        if self.epoch_state.epoch != session_metadata.dealer_epoch {
            warn!(
                "[DKG] event (from epoch {}) not for current epoch ({}), ignoring",
                session_metadata.dealer_epoch, self.epoch_state.epoch
            );
            return Ok(());
        }
        self.setup_deal_broadcast(start_time_us, &session_metadata)
            .await
    }

    /// Process an RPC request from DKG peers.
    async fn process_peer_rpc_msg(&mut self, req: IncomingRpcRequest) -> Result<()> {
        let IncomingRpcRequest {
            msg,
            mut response_sender,
            ..
        } = req;
        ensure!(
            msg.epoch() == self.epoch_state.epoch,
            "[DKG] msg not for current epoch"
        );
        let response = match (&self.state, &msg) {
            (InnerState::Finished { my_transcript, .. }, DKGMessage::TranscriptRequest(_))
            | (InnerState::InProgress { my_transcript, .. }, DKGMessage::TranscriptRequest(_)) => {
                Ok(DKGMessage::TranscriptResponse(my_transcript.clone()))
            },
            _ => Err(anyhow!(
                "[DKG] msg {:?} unexpected in state {:?}",
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
