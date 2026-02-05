// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::{
        agg_subtrx_producer,
        missing_transcript_fetcher::MissingTranscriptFetcher,
        subtrx_cert_producer,
        types::{
            AggregatedSubtranscript, CertifiedAggregatedSubtranscript, MissingTranscriptRequest,
            MissingTranscriptResponse,
        },
    },
    counters,
    network::{IncomingRpcRequest, NetworkSender, RpcResponseSender},
    types::{ChunkyDKGSubtranscriptSignatureRequest, ChunkyDKGSubtranscriptSignatureResponse},
    DKGMessage,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::{hash::CryptoHash, SigningKey, Uniform};
use aptos_dkg::pvss::{
    traits::{transcript::HasAggregatableSubtranscript, Aggregatable},
    Player,
};
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::{debug, error, info, warn};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    dkg::{
        chunky_dkg::{
            CertifiedAggregatedChunkySubtranscript, ChunkyDKG, ChunkyDKGConfig,
            ChunkyDKGSessionMetadata, ChunkyDKGSessionState, ChunkyDKGStartEvent,
            ChunkyDKGTranscript, ChunkyInputSecret, ChunkySubtranscript, ChunkyTranscript,
            DealerPrivateKey, DealerPublicKey,
        },
        DKGTranscriptMetadata,
    },
    epoch_state::EpochState,
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use futures_channel::oneshot;
use futures_util::{future::AbortHandle, FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use rand::{prelude::StdRng, thread_rng, SeedableRng};
use std::{
    collections::{HashMap, HashSet},
    mem,
    sync::Arc,
    time::Duration,
};
use tokio_retry::strategy::ExponentialBackoff;

#[allow(dead_code)]
#[derive(Debug, Default)]
enum InnerState {
    #[default]
    Init,
    AwaitSubtranscriptAggregation {
        start_time: Duration,
        my_transcript: ChunkyDKGTranscript,
        dkg_config: ChunkyDKGConfig,
        abort_handle: AbortHandle,
    },
    AwaitAggregatedSubtranscriptCertification {
        start_time: Duration,
        my_transcript: ChunkyDKGTranscript,
        aggregated_subtranscript: AggregatedSubtranscript,
        dkg_config: ChunkyDKGConfig,
        abort_handle: AbortHandle,
    },
    Finished {
        vtxn_guard: TxnGuard,
        start_time: Duration,
        my_transcript: ChunkyDKGTranscript,
        proposed: bool,
    },
}

impl InnerState {
    #[allow(dead_code)]
    fn variant_name(&self) -> &str {
        match self {
            InnerState::Init => "NotStarted",
            InnerState::AwaitSubtranscriptAggregation { .. } => "AwaitSubtranscriptAggregation",
            InnerState::AwaitAggregatedSubtranscriptCertification { .. } => {
                "AwaitAggregatedSubtranscriptCertification"
            },
            InnerState::Finished { .. } => "Finished",
        }
    }
}

#[allow(dead_code)]
pub struct ChunkyDKGManager {
    ssk: Arc<DealerPrivateKey>,
    spk: Arc<DealerPublicKey>,
    my_index: usize,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,

    vtxn_pool: VTxnPoolState,
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,

    agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscript>>,
    certified_subtrx_tx: Option<aptos_channel::Sender<(), CertifiedAggregatedSubtranscript>>,

    // When we put vtxn in the pool, we also put a copy of this so later pool can notify us.
    pull_notification_tx: aptos_channel::Sender<(), Arc<ValidatorTransaction>>,
    pull_notification_rx: aptos_channel::Receiver<(), Arc<ValidatorTransaction>>,

    // Shared map to track transcripts received from each recipient
    received_transcripts: Arc<Mutex<HashMap<AccountAddress, ChunkyTranscript>>>,

    // Control states.
    stopped: bool,
    state: InnerState,
}

#[allow(dead_code)]
impl ChunkyDKGManager {
    pub fn new(
        ssk: Arc<DealerPrivateKey>,
        spk: Arc<DealerPublicKey>,
        my_index: usize,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        vtxn_pool: VTxnPoolState,
        reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
        network_sender: Arc<NetworkSender>,
    ) -> Self {
        let (pull_notification_tx, pull_notification_rx) =
            aptos_channel::new(QueueStyle::KLAST, 1, None);
        Self {
            ssk,
            spk,
            my_addr,
            my_index,
            epoch_state,
            vtxn_pool,
            reliable_broadcast,
            network_sender,
            agg_subtrx_tx: None,
            certified_subtrx_tx: None,
            pull_notification_tx,
            pull_notification_rx,
            received_transcripts: Arc::new(Mutex::new(HashMap::new())),
            stopped: false,
            state: InnerState::Init,
        }
    }

    pub async fn run(
        mut self,
        in_progress_session: Option<ChunkyDKGSessionState>,
        mut dkg_start_event_rx: aptos_channel::Receiver<(), ChunkyDKGStartEvent>,
        mut rpc_msg_rx: aptos_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[ChunkyDKG] ChunkyDKGManager started."
        );
        let mut interval = tokio::time::interval(Duration::from_millis(5000));

        let (agg_subtrx_tx, mut agg_subtrx_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.agg_subtrx_tx = Some(agg_subtrx_tx);

        let (certified_subtrx_tx, mut certified_subtrx_rx) =
            aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.certified_subtrx_tx = Some(certified_subtrx_tx);

        if let Some(session_state) = in_progress_session {
            let ChunkyDKGSessionState {
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
                        .map_err(|e|anyhow!("[ChunkyDKG] process_dkg_start_event failed: {e}"))
                },
                (_sender, msg) = rpc_msg_rx.select_next_some() => {
                    self.process_peer_rpc_msg(msg)
                        .await
                        .map_err(|e|anyhow!("[ChunkyDKG] process_peer_rpc_msg failed: {e}"))
                },
                agg_subtranscript = agg_subtrx_rx.select_next_some() => {
                    self.process_aggregated_subtranscript(agg_subtranscript)
                        .await
                        .map_err(|e|anyhow!("[ChunkyDKG] process_aggregated_subtranscript failed: {e}"))
                },
                certified_transcript = certified_subtrx_rx.select_next_some() => {
                    self.process_certified_aggregated_subtranscript(certified_transcript)
                        .await
                        .map_err(|e|anyhow!("[ChunkyDKG] process_certified_aggregated_subtranscript failed: {e}"))
                },
                dkg_txn = self.pull_notification_rx.select_next_some() => {
                    self.process_dkg_txn_pulled_notification(dkg_txn)
                        .await
                        .map_err(|e|anyhow!("[ChunkyDKG] process_dkg_txn_pulled_notification failed: {e}"))
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
                    "[ChunkyDKG] ChunkyDKGManager handling error: {e}"
                );
            }
        }
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[ChunkyDKG] ChunkyDKGManager finished."
        );
    }

    fn observe(&self) -> Result<()> {
        debug!("[ChunkyDKG] chunky_dkg_manager_state={:?}", self.state);
        Ok(())
    }

    /// On a CLOSE command from epoch manager, do clean-up.
    fn process_close_cmd(&mut self, ack_tx: Option<oneshot::Sender<()>>) -> Result<()> {
        self.stopped = true;

        match std::mem::take(&mut self.state) {
            InnerState::Init => {},
            InnerState::AwaitSubtranscriptAggregation { abort_handle, .. } => {
                abort_handle.abort();
            },
            InnerState::AwaitAggregatedSubtranscriptCertification { abort_handle, .. } => {
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
                counters::observe_chunky_dkg_stage(start_time, self.my_addr, "epoch_change");
                info!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr,
                    secs_since_dkg_start = secs_since_dkg_start,
                    "[ChunkyDKG] txn executed and entering new epoch.",
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
                    counters::observe_chunky_dkg_stage(*start_time, self.my_addr, "proposed");
                    info!(
                        epoch = self.epoch_state.epoch,
                        my_addr = self.my_addr,
                        secs_since_dkg_start = secs_since_dkg_start,
                        "[ChunkyDKG] aggregated transcript proposed by consensus.",
                    );
                }
                Ok(())
            },
            _ => {
                bail!("[ChunkyDKG] pull notification only expected in finished state");
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
        dkg_session_metadata: &ChunkyDKGSessionMetadata,
    ) -> Result<()> {
        ensure!(
            matches!(&self.state, InnerState::Init),
            "transcript already dealt"
        );
        let dkg_start_time = Duration::from_micros(start_time_us);
        let deal_start = duration_since_epoch();
        let secs_since_dkg_start = deal_start.as_secs_f64() - dkg_start_time.as_secs_f64();
        counters::observe_chunky_dkg_stage(dkg_start_time, self.my_addr, "deal_start");
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            secs_since_dkg_start = secs_since_dkg_start,
            "[ChunkyDKG] Deal transcript started.",
        );

        let dkg_config = ChunkyDKG::generate_config(dkg_session_metadata);

        let mut rng = StdRng::from_rng(thread_rng()).unwrap();
        let input_secret = ChunkyInputSecret::generate(&mut rng);

        let dealer = Player { id: self.my_index };
        let session_id = dkg_session_metadata;
        // TODO: Persist the transcript? I don't think it's needed, but rethink later.
        let trx = ChunkyDKG::deal(
            &dkg_config,
            &self.ssk,
            &self.spk,
            &input_secret,
            session_id,
            &dealer,
            &mut rng,
        );

        let my_transcript = ChunkyDKGTranscript::new(
            self.epoch_state.epoch,
            self.my_addr,
            bcs::to_bytes(&trx).map_err(|e| anyhow!("transcript serialization error: {e}"))?,
        );

        let deal_finish = duration_since_epoch();
        let secs_since_dkg_start = deal_finish.as_secs_f64() - dkg_start_time.as_secs_f64();
        counters::observe_chunky_dkg_stage(dkg_start_time, self.my_addr, "deal_finish");
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            secs_since_dkg_start = secs_since_dkg_start,
            "[ChunkyDKG] Deal transcript finished.",
        );

        // Get dealer public keys from session metadata
        let spks: Vec<DealerPublicKey> = dkg_session_metadata
            .dealer_consensus_infos_cloned()
            .into_iter()
            .map(|info| info.public_key)
            .collect();

        // Start aggregation producer
        let abort_handle = agg_subtrx_producer::start_subtranscript_aggregation(
            self.reliable_broadcast.clone(),
            self.epoch_state.clone(),
            self.my_addr,
            dkg_config.clone(),
            spks,
            dkg_start_time,
            self.agg_subtrx_tx.as_ref().cloned(),
            self.received_transcripts.clone(),
        );

        // Switch to the next stage.
        self.state = InnerState::AwaitSubtranscriptAggregation {
            start_time: dkg_start_time,
            my_transcript,
            abort_handle,
            dkg_config,
        };

        Ok(())
    }

    /// On a locally aggregated transcript, start validation and update inner states.
    async fn process_aggregated_subtranscript(
        &mut self,
        aggregated_subtranscript: AggregatedSubtranscript,
    ) -> Result<()> {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[ChunkyDKG] Processing locally aggregated subtranscript."
        );
        ensure!(
            matches!(self.state, InnerState::AwaitSubtranscriptAggregation { .. }),
            "[ChunkyDKG] aggregated transcript only expected during DKG"
        );
        let InnerState::AwaitSubtranscriptAggregation {
            start_time,
            my_transcript,
            dkg_config,
            ..
        } = std::mem::take(&mut self.state)
        else {
            unreachable!("The ensure! above must take care of this");
        };

        counters::observe_chunky_dkg_stage(start_time, self.my_addr, "agg_transcript_ready");

        let abort_handle = subtrx_cert_producer::start_chunky_subtranscript_certification(
            self.reliable_broadcast.clone(),
            start_time,
            self.my_addr,
            self.epoch_state.clone(),
            dkg_config.clone(),
            aggregated_subtranscript.clone(),
            self.certified_subtrx_tx.clone(),
        );

        self.state = InnerState::AwaitAggregatedSubtranscriptCertification {
            start_time,
            my_transcript,
            aggregated_subtranscript,
            dkg_config,
            abort_handle,
        };

        Ok(())
    }

    async fn process_certified_aggregated_subtranscript(
        &mut self,
        certified_agg_subtrx: CertifiedAggregatedSubtranscript,
    ) -> Result<()> {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[ChunkyDKG] Processing validated aggregated transcript."
        );

        ensure!(
            matches!(
                self.state,
                InnerState::AwaitAggregatedSubtranscriptCertification { .. }
            ),
            "[ChunkyDKG] aggregated transcript only expected during DKG"
        );
        let InnerState::AwaitAggregatedSubtranscriptCertification {
            start_time,
            my_transcript,
            ..
        } = mem::take(&mut self.state)
        else {
            unreachable!("The ensure! above must disallow this case");
        };

        let CertifiedAggregatedSubtranscript {
            aggregated_subtranscript,
            aggregate_signature,
        } = certified_agg_subtrx;

        counters::observe_chunky_dkg_stage(start_time, self.my_addr, "agg_subtrx_certified");

        let txn = ValidatorTransaction::ChunkyDKGResult(CertifiedAggregatedChunkySubtranscript {
            metadata: DKGTranscriptMetadata {
                epoch: self.epoch_state.epoch,
                author: self.my_addr,
            },
            transcript_bytes: bcs::to_bytes(&aggregated_subtranscript)
                .map_err(|e| anyhow!("transcript serialization error: {e}"))?,
            signature: aggregate_signature,
        });
        // TODO(ibalajiarun): Derive Topic from txn
        let vtxn_guard = self.vtxn_pool.put(
            Topic::ChunkyDKG,
            Arc::new(txn),
            Some(self.pull_notification_tx.clone()),
        );
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[ChunkyDKG] aggregated transcript put into vtxn pool."
        );
        self.state = InnerState::Finished {
            vtxn_guard,
            start_time,
            my_transcript,
            proposed: false,
        };
        Ok(())
    }

    async fn process_dkg_start_event(&mut self, event: ChunkyDKGStartEvent) -> Result<()> {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[ChunkyDKG] Processing DKGStart event."
        );
        let ChunkyDKGStartEvent {
            session_metadata,
            start_time_us,
        } = event;
        ensure!(
            matches!(&self.state, InnerState::Init),
            "[ChunkyDKG] dkg already started"
        );
        if self.epoch_state.epoch != session_metadata.dealer_epoch {
            warn!(
                "[ChunkyDKG] event (from epoch {}) not for current epoch ({}), ignoring",
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
            response_sender,
            sender,
        } = req;
        ensure!(
            msg.epoch() == self.epoch_state.epoch,
            "[ChunkyDKG] msg not for current epoch"
        );
        match &msg {
            DKGMessage::ChunkyTranscriptRequest(_) => {
                self.handle_chunky_transcript_request_rpc(response_sender)?;
            },
            DKGMessage::SubtranscriptSignatureRequest(req) => {
                self.process_subtranscript_signature_request_rpc(
                    sender,
                    req.clone(),
                    response_sender,
                )?;
            },
            DKGMessage::MissingTranscriptRequest(req) => {
                self.handle_missing_transcript_request_rpc(req.clone(), response_sender)?;
            },
            _ => {
                return Err(anyhow!(
                    "[ChunkyDKG] msg {:?} unexpected in state {:?}",
                    msg.name(),
                    self.state.variant_name()
                ));
            },
        }
        Ok(())
    }

    /// Process a transcript request RPC message.
    fn handle_chunky_transcript_request_rpc(
        &self,
        mut response_sender: Box<dyn RpcResponseSender>,
    ) -> Result<()> {
        let my_transcript = match &self.state {
            InnerState::Finished { my_transcript, .. }
            | InnerState::AwaitSubtranscriptAggregation { my_transcript, .. }
            | InnerState::AwaitAggregatedSubtranscriptCertification { my_transcript, .. } => {
                my_transcript
            },
            _ => {
                bail!(
                    "[ChunkyDKG] transcript request unexpected in state {:?}",
                    self.state.variant_name()
                );
            },
        };
        let response = DKGMessage::ChunkyTranscriptResponse(ChunkyDKGTranscript::new(
            my_transcript.metadata.epoch,
            my_transcript.metadata.author,
            my_transcript.transcript_bytes.clone(),
        ));
        response_sender.send(Ok(response));
        Ok(())
    }

    /// Process a missing transcript request RPC message.
    /// Looks up transcript in received_transcripts and returns it via the response_sender.
    fn handle_missing_transcript_request_rpc(
        &self,
        req: MissingTranscriptRequest,
        mut response_sender: Box<dyn RpcResponseSender>,
    ) -> Result<()> {
        let received_transcripts = self.received_transcripts.lock();
        let dealer_addr = req.missing_dealer;

        let response = match received_transcripts.get(&dealer_addr) {
            Some(transcript) => {
                let bytes = bcs::to_bytes(transcript)
                    .map_err(|e| anyhow!("transcript serialization error: {e}"))?;
                Ok(DKGMessage::MissingTranscriptResponse(
                    MissingTranscriptResponse::new(ChunkyDKGTranscript::new(
                        req.dealer_epoch,
                        dealer_addr,
                        bytes,
                    )),
                ))
            },
            None => Err(anyhow!("Transcript not found for dealer {}", dealer_addr)),
        };
        response_sender.send(response);
        Ok(())
    }

    /// Process a subtranscript validation request RPC message.
    /// Spawns a tokio task to handle the computation and respond via the response_sender.
    fn process_subtranscript_signature_request_rpc(
        &self,
        sender: AccountAddress,
        req: ChunkyDKGSubtranscriptSignatureRequest,
        mut response_sender: Box<dyn RpcResponseSender>,
    ) -> Result<()> {
        let (aggregated_transcript, dkg_config) = match &self.state {
            InnerState::AwaitAggregatedSubtranscriptCertification {
                aggregated_subtranscript: aggregated_transcript,
                dkg_config,
                ..
            } => (aggregated_transcript.clone(), dkg_config.clone()),
            _ => {
                bail!(
                    "[ChunkyDKG] subtranscript validation request unexpected in state {:?}",
                    self.state.variant_name()
                );
            },
        };

        // Spawn a tokio task to handle the validation computation
        let received_transcripts = self.received_transcripts.clone();
        let epoch_state = self.epoch_state.clone();
        let ssk = self.ssk.clone();
        let my_addr = self.my_addr;
        let network_sender = self.network_sender.clone();
        // TODO(ibalajiarun): Track the handle and cancel task properly
        tokio::spawn(async move {
            let response = Self::handle_subtranscript_signature_request(
                sender,
                req,
                aggregated_transcript,
                dkg_config,
                ssk,
                my_addr,
                received_transcripts,
                epoch_state,
                network_sender,
            )
            .await;
            response_sender.send(response);
        });

        Ok(())
    }

    /// Handle subtranscript validation computation.
    async fn handle_subtranscript_signature_request(
        sender: AccountAddress,
        req: ChunkyDKGSubtranscriptSignatureRequest,
        local_aggregated_transcript: AggregatedSubtranscript,
        dkg_config: ChunkyDKGConfig,
        ssk: Arc<DealerPrivateKey>,
        _my_addr: AccountAddress,
        received_transcripts: Arc<Mutex<HashMap<AccountAddress, ChunkyTranscript>>>,
        epoch_state: Arc<EpochState>,
        network_sender: Arc<NetworkSender>,
    ) -> Result<DKGMessage> {
        // In the miniscule chance that the locally aggregated subtranscript is the same as the
        // remote transcript, we can just sign and return immediately.
        if local_aggregated_transcript.hash() == req.subtranscript_hash {
            let signature = ssk
                .sign(&local_aggregated_transcript)
                .map_err(|e| anyhow!("failed to sign subtranscript validation: {:?}", e))?;

            // Build and send a response message
            let response = DKGMessage::SubtranscriptSignatureResponse(
                ChunkyDKGSubtranscriptSignatureResponse::new(
                    req.dealer_epoch,
                    req.subtranscript_hash,
                    signature,
                ),
            );

            return Ok(response);
        }

        // Convert Player dealers to AccountAddress using validator indices
        let dealer_addresses: Vec<AccountAddress> = req
            .aggregated_subtrx_dealers
            .iter()
            .filter_map(|player| {
                epoch_state
                    .verifier
                    .get_ordered_account_addresses()
                    .get(player.id)
                    .copied()
            })
            .collect();
        ensure!(dealer_addresses.len() == req.aggregated_subtrx_dealers.len());

        let required_dealers: HashSet<AccountAddress> = dealer_addresses.iter().cloned().collect();
        let (mut subtranscripts, missing_dealers) = {
            // Get received transcripts from shared storage
            let received_transcripts_map = received_transcripts.lock();
            let available_addresses: HashSet<AccountAddress> =
                received_transcripts_map.keys().cloned().collect();

            // Collect transcripts for all required dealers
            let subtranscripts: Vec<ChunkySubtranscript> = required_dealers
                .iter()
                .filter_map(|addr| {
                    received_transcripts_map
                        .get(addr)
                        .map(|tx| tx.get_subtranscript())
                })
                .collect();

            // Find missing dealers
            let missing_dealers: Vec<AccountAddress> = required_dealers
                .difference(&available_addresses)
                .cloned()
                .collect();

            (subtranscripts, missing_dealers)
        };

        // Fetch missing transcripts from network if needed
        if !missing_dealers.is_empty() {
            // Create a fetcher and fetch missing transcripts from the sender
            let fetcher = MissingTranscriptFetcher::new(
                sender,
                req.dealer_epoch,
                missing_dealers.clone(),
                Duration::from_secs(10), // RPC timeout
                dkg_config.clone(),
            );

            let fetched_transcripts = fetcher.run(network_sender).await?;

            for t in fetched_transcripts.into_values() {
                subtranscripts.push(t.get_subtranscript());
            }
        }

        ensure!(
            !subtranscripts.is_empty(),
            "No transcripts found for required dealers"
        );
        ensure!(
            subtranscripts.len() == dealer_addresses.len(),
            "Not enough subtranscripts"
        );

        // Aggregate subtranscripts using the same logic as in chunky_agg_trx_producer
        let mut recomputed_subtranscript = subtranscripts.remove(0);
        for other in subtranscripts.iter() {
            recomputed_subtranscript
                .aggregate_with(&dkg_config.threshold_config, other)
                .context("failed to aggregate subtranscripts")?;
        }
        let recomputed_aggsubtranscript = AggregatedSubtranscript {
            subtranscript: recomputed_subtranscript,
            dealers: req.aggregated_subtrx_dealers.clone(),
        };

        // Verify the hash matches
        ensure!(
            recomputed_aggsubtranscript.hash() == req.subtranscript_hash,
            "subtranscript hash mismatch in validation request"
        );

        // Sign over the subtranscript hash
        let signature = ssk
            .sign(&recomputed_aggsubtranscript)
            .map_err(|e| anyhow!("failed to sign subtranscript validation: {:?}", e))?;

        // Build and send a response message
        let response = DKGMessage::SubtranscriptSignatureResponse(
            ChunkyDKGSubtranscriptSignatureResponse::new(
                req.dealer_epoch,
                req.subtranscript_hash,
                signature,
            ),
        );

        Ok(response)
    }
}
