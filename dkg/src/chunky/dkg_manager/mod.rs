// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::{
        agg_subtrx_producer,
        missing_transcript_fetcher::TranscriptFetcher,
        subtrx_cert_producer,
        types::{
            AggregatedSubtranscriptWithHashes, CertifiedAggregatedSubtranscript,
            MissingTranscriptRequest, MissingTranscriptResponse,
        },
        DIGEST_KEY,
    },
    counters, monitor,
    network::{IncomingRpcRequest, NetworkSender, RpcResponseSender},
    types::{ChunkyDKGSubtranscriptSignatureRequest, ChunkyDKGSubtranscriptSignatureResponse},
    DKGMessage,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::{hash::CryptoHash, SigningKey, Uniform};
use aptos_dkg::pvss::{
    traits::transcript::{Aggregatable, HasAggregatableSubtranscript},
    Player,
};
use aptos_infallible::{duration_since_epoch, RwLock};
use aptos_logger::{debug, error, info, warn};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_types::{
    dkg::{
        chunky_dkg::{
            AggregatedSubtranscript, CertifiedAggregatedChunkySubtranscript,
            CertifiedChunkyDKGOutput, ChunkyDKGSession, ChunkyDKGSessionMetadata,
            ChunkyDKGSessionState, ChunkyDKGStartEvent, ChunkyDKGTranscript, ChunkyInputSecret,
            ChunkySubtranscript, ChunkyTranscript, DealerPrivateKey, DealerPublicKey,
        },
        DKGTranscriptMetadata,
    },
    epoch_state::EpochState,
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use futures_channel::oneshot;
use futures_util::{
    future::{AbortHandle, Abortable},
    FutureExt, StreamExt,
};
use move_core_types::account_address::AccountAddress;
use rand::{prelude::StdRng, thread_rng, SeedableRng};
use std::{collections::HashMap, fmt, mem, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

#[cfg(test)]
mod tests;

#[derive(Default)]
enum InnerState {
    #[default]
    Init,
    AwaitSubtranscriptAggregation {
        start_time: Duration,
        my_transcript: Arc<ChunkyDKGTranscript>,
        dkg_config: Arc<ChunkyDKGSession>,
        _abort_guard: DropGuard,
    },
    AwaitAggregatedSubtranscriptCertification {
        start_time: Duration,
        my_transcript: Arc<ChunkyDKGTranscript>,
        aggregated_subtranscript: Arc<AggregatedSubtranscript>,
        dkg_config: Arc<ChunkyDKGSession>,
        _abort_guard: DropGuard,
    },
    Finished {
        vtxn_guard: TxnGuard,
        start_time: Duration,
        my_transcript: Arc<ChunkyDKGTranscript>,
        proposed: bool,
    },
}

impl fmt::Debug for InnerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.variant_name())
    }
}

impl InnerState {
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

pub struct ChunkyDKGManager {
    ssk: Arc<DealerPrivateKey>,
    spk: Arc<DealerPublicKey>,
    my_index: usize,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,

    vtxn_pool: VTxnPoolState,
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,

    agg_subtrx_tx: Option<aptos_channel::Sender<(), AggregatedSubtranscriptWithHashes>>,
    certified_subtrx_tx: Option<aptos_channel::Sender<(), CertifiedAggregatedSubtranscript>>,

    // When we put vtxn in the pool, we also put a copy of this so later pool can notify us.
    pull_notification_tx: aptos_channel::Sender<(), Arc<ValidatorTransaction>>,
    pull_notification_rx: aptos_channel::Receiver<(), Arc<ValidatorTransaction>>,

    // Shared map to track transcripts received from each recipient.
    // RwLock: aggregation task writes; main loop and handler tasks only read.
    // Values are Arc-wrapped to allow cheap clone-out for lock-free serialization/hashing.
    received_transcripts: Arc<RwLock<HashMap<AccountAddress, Arc<ChunkyTranscript>>>>,

    // Guards for spawned RPC handler tasks, keyed by requesting validator's address.
    // If a new request arrives from the same sender, the old handler is aborted and replaced.
    rpc_handler_guards: HashMap<AccountAddress, DropGuard>,

    // Control states.
    stopped: bool,
    state: InnerState,
}

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
        // Signatures are created with ssk but verified with the consensus BLS key from
        // epoch_state.verifier. Assert they match to catch key misconfiguration early.
        assert!(
            epoch_state.verifier.get_public_key(&my_addr).as_ref() == Some(spk.as_ref()),
            "[ChunkyDKG] spk does not match consensus public key for my_addr"
        );
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
            received_transcripts: Arc::new(RwLock::new(HashMap::new())),
            rpc_handler_guards: HashMap::new(),
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
                    monitor!("chunky_mgr_process_dkg_start_event",
                        self.process_dkg_start_event(dkg_start_event)
                            .await
                            .map_err(|e|anyhow!("[ChunkyDKG] process_dkg_start_event failed: {e}"))
                    )
                },
                (_sender, msg) = rpc_msg_rx.select_next_some() => {
                    monitor!("chunky_mgr_process_peer_rpc_msg",
                        self.process_peer_rpc_msg(msg)
                            .await
                            .map_err(|e|anyhow!("[ChunkyDKG] process_peer_rpc_msg failed: {e}"))
                    )
                },
                agg_subtranscript = agg_subtrx_rx.select_next_some() => {
                    monitor!("chunky_mgr_process_aggregated_subtranscript",
                        self.process_aggregated_subtranscript(agg_subtranscript)
                            .await
                            .map_err(|e|anyhow!("[ChunkyDKG] process_aggregated_subtranscript failed: {e}"))
                    )
                },
                certified_transcript = certified_subtrx_rx.select_next_some() => {
                    monitor!("chunky_mgr_process_certified_aggregated_subtranscript",
                        self.process_certified_aggregated_subtranscript(certified_transcript).await
                            .map_err(|e|anyhow!("[ChunkyDKG] process_certified_aggregated_subtranscript failed: {e}"))
                    )
                },
                dkg_txn = self.pull_notification_rx.select_next_some() => {
                    monitor!("chunky_mgr_process_dkg_txn_pulled_notification",
                        self.process_dkg_txn_pulled_notification(dkg_txn)
                            .await
                            .map_err(|e|anyhow!("[ChunkyDKG] process_dkg_txn_pulled_notification failed: {e}"))
                    )
                },
                close_req = close_rx.select_next_some() => {
                    monitor!("chunky_mgr_process_close_cmd",
                        self.process_close_cmd(close_req.ok())
                    )
                },
                _ = interval.tick().fuse() => {
                    monitor!("chunky_mgr_observe",
                        self.observe()
                    )
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
        self.rpc_handler_guards.clear();

        match std::mem::take(&mut self.state) {
            InnerState::Init => {},
            InnerState::AwaitSubtranscriptAggregation { .. } => {},
            InnerState::AwaitAggregatedSubtranscriptCertification { .. } => {},
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

        let dkg_config = ChunkyDKGSession::new(dkg_session_metadata);
        let dkg_config_clone = dkg_config.clone();
        let ssk_clone = self.ssk.clone();
        let spk_clone = self.spk.clone();
        let my_index = self.my_index;
        let dkg_session_metadata_clone = dkg_session_metadata.clone();

        let trx = tokio::task::spawn_blocking(move || {
            let mut rng = StdRng::from_rng(thread_rng()).unwrap();
            let input_secret = ChunkyInputSecret::generate(&mut rng);

            let dealer = Player { id: my_index };
            let session_id = dkg_session_metadata_clone;

            dkg_config_clone.deal(
                &ssk_clone,
                &spk_clone,
                &input_secret,
                &session_id,
                &dealer,
                &mut rng,
            )
        })
        .await?;

        let transcript_bytes =
            bcs::to_bytes(&trx).map_err(|e| anyhow!("transcript serialization error: {e}"))?;
        counters::CHUNKY_DKG_OBJECT_SIZE_BYTES
            .with_label_values(&["dealer_transcript"])
            .observe(transcript_bytes.len() as f64);
        let my_transcript = Arc::new(ChunkyDKGTranscript::new(
            self.epoch_state.epoch,
            self.my_addr,
            transcript_bytes,
        ));

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
            _abort_guard: DropGuard::new(abort_handle),
            dkg_config,
        };

        Ok(())
    }

    /// On a locally aggregated transcript, start validation and update inner states.
    async fn process_aggregated_subtranscript(
        &mut self,
        agg_with_hashes: AggregatedSubtranscriptWithHashes,
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

        let AggregatedSubtranscriptWithHashes {
            aggregated_subtranscript,
            dealer_transcript_hashes,
        } = agg_with_hashes;

        counters::observe_chunky_dkg_stage(start_time, self.my_addr, "agg_transcript_ready");

        let aggregated_subtranscript = Arc::new(aggregated_subtranscript);

        let abort_handle = subtrx_cert_producer::start_chunky_subtranscript_certification(
            self.reliable_broadcast.clone(),
            start_time,
            self.my_addr,
            self.epoch_state.clone(),
            aggregated_subtranscript.clone(),
            dealer_transcript_hashes,
            self.certified_subtrx_tx.clone(),
        );

        self.state = InnerState::AwaitAggregatedSubtranscriptCertification {
            start_time,
            my_transcript,
            aggregated_subtranscript,
            dkg_config,
            _abort_guard: DropGuard::new(abort_handle),
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

        info!("[ChunkyDKG] deriving encryption key");
        // Derive encryption key from subtranscript + DigestKey.
        // Heavy pairing-based crypto — run off the main loop.
        let (encryption_key_bytes, transcript_bytes) = tokio::task::spawn_blocking(move || {
            let digest_key = DIGEST_KEY
                .as_ref()
                .ok_or_else(|| anyhow!("DigestKey not available; cannot derive encryption key"))?;
            let key = aggregated_subtranscript.derive_encryption_key_bytes(digest_key.tau_g2)?;
            let bytes = bcs::to_bytes(&aggregated_subtranscript)
                .map_err(|e| anyhow!("transcript serialization error: {e}"))?;
            counters::CHUNKY_DKG_OBJECT_SIZE_BYTES
                .with_label_values(&["aggregated_subtranscript"])
                .observe(bytes.len() as f64);
            counters::CHUNKY_DKG_OBJECT_SIZE_BYTES
                .with_label_values(&["encryption_key"])
                .observe(key.len() as f64);
            Ok::<_, anyhow::Error>((key, bytes))
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking join error: {e}"))??;

        info!("[ChunkyDKG] forming certified_transcript struct");
        counters::CHUNKY_DKG_OBJECT_SIZE_BYTES
            .with_label_values(&["certified_transcript"])
            .observe(transcript_bytes.len() as f64);
        let certified_transcript = CertifiedAggregatedChunkySubtranscript {
            metadata: DKGTranscriptMetadata {
                epoch: self.epoch_state.epoch,
                author: self.my_addr,
            },
            transcript_bytes,
            signature: aggregate_signature,
        };

        let txn = ValidatorTransaction::ChunkyDKGResult(CertifiedChunkyDKGOutput {
            certified_transcript,
            encryption_key: encryption_key_bytes,
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
                monitor!(
                    "chunky_mgr_handle_chunky_transcript_request_rpc",
                    self.handle_chunky_transcript_request_rpc(response_sender)
                )?;
            },
            DKGMessage::SubtranscriptSignatureRequest(req) => {
                monitor!(
                    "chunky_mgr_process_subtranscript_signature_request_rpc",
                    self.process_subtranscript_signature_request_rpc(
                        sender,
                        req.clone(),
                        response_sender,
                    )
                )?;
            },
            DKGMessage::MissingTranscriptRequest(req) => {
                monitor!(
                    "chunky_mgr_handle_missing_transcript_request_rpc",
                    self.handle_missing_transcript_request_rpc(req.clone(), response_sender)
                )?;
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
            InnerState::Init => {
                response_sender.send(Err(anyhow!(
                    "[ChunkyDKG] transcript request unexpected in state {:?}",
                    self.state.variant_name()
                )));
                return Ok(());
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
        let missing_dealer = req.missing_dealer;
        let maybe_transcript = self
            .received_transcripts
            .read()
            .get(&missing_dealer)
            .cloned();
        let response = match maybe_transcript {
            Some(transcript) => {
                let bytes = bcs::to_bytes(transcript.as_ref())
                    .map_err(|e| anyhow!("transcript serialization error: {e}"))?;
                Ok(DKGMessage::MissingTranscriptResponse(
                    MissingTranscriptResponse::new(ChunkyDKGTranscript::new(
                        req.dealer_epoch,
                        missing_dealer,
                        bytes,
                    )),
                ))
            },
            None => Err(anyhow!(
                "Transcript not found for dealer {}",
                missing_dealer,
            )),
        };
        response_sender.send(response);
        Ok(())
    }

    /// Process a subtranscript validation request RPC message.
    /// Spawns a tokio task to handle the computation and respond via the response_sender.
    fn process_subtranscript_signature_request_rpc(
        &mut self,
        sender: AccountAddress,
        req: ChunkyDKGSubtranscriptSignatureRequest,
        mut response_sender: Box<dyn RpcResponseSender>,
    ) -> Result<()> {
        let (aggregated_transcript, dkg_config) = match &self.state {
            InnerState::AwaitAggregatedSubtranscriptCertification {
                aggregated_subtranscript: aggregated_transcript,
                dkg_config,
                ..
            } => (Arc::clone(aggregated_transcript), Arc::clone(dkg_config)),
            _ => {
                // Send error response instead of dropping response_sender.
                // Drop = timeout = exponential backoff blowup on the requester side.
                // Error = quick retry.
                response_sender.send(Err(anyhow!(
                    "[ChunkyDKG] not ready for signature requests in state {:?}",
                    self.state.variant_name()
                )));
                return Ok(());
            },
        };

        // Spawn a tokio task to handle the validation computation.
        // Keyed by sender — if a handler for this sender already exists, the old DropGuard is
        // dropped (aborting the previous task) and replaced with the new one. This ensures the
        // latest request from each validator is always served.
        let received_transcripts = self.received_transcripts.clone();
        let epoch_state = self.epoch_state.clone();
        let ssk = self.ssk.clone();
        let my_addr = self.my_addr;
        let epoch = self.epoch_state.epoch;
        let network_sender = self.network_sender.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(
            async move {
                const HANDLER_TIMEOUT: Duration = Duration::from_secs(30);
                let result = tokio::time::timeout(
                    HANDLER_TIMEOUT,
                    Self::handle_subtranscript_signature_request(
                        sender,
                        req,
                        aggregated_transcript,
                        dkg_config,
                        ssk,
                        my_addr,
                        received_transcripts,
                        epoch_state,
                        network_sender,
                    ),
                )
                .await;
                let response = match result {
                    Ok(r) => r,
                    Err(_) => {
                        warn!(
                            epoch = epoch,
                            sender = sender,
                            "[ChunkyDKG] signature request handler timed out after {}s",
                            HANDLER_TIMEOUT.as_secs()
                        );
                        Err(anyhow!(
                            "signature request handler timed out after {}s",
                            HANDLER_TIMEOUT.as_secs()
                        ))
                    },
                };
                response_sender.send(response);
            },
            abort_registration,
        ));
        // Insert replaces any existing handler for this sender, aborting the old task.
        self.rpc_handler_guards
            .insert(sender, DropGuard::new(abort_handle));

        Ok(())
    }

    /// Handle subtranscript validation computation.
    async fn handle_subtranscript_signature_request(
        sender: AccountAddress,
        req: ChunkyDKGSubtranscriptSignatureRequest,
        local_aggregated_transcript: Arc<AggregatedSubtranscript>,
        dkg_config: Arc<ChunkyDKGSession>,
        ssk: Arc<DealerPrivateKey>,
        _my_addr: AccountAddress,
        received_transcripts: Arc<RwLock<HashMap<AccountAddress, Arc<ChunkyTranscript>>>>,
        epoch_state: Arc<EpochState>,
        network_sender: Arc<NetworkSender>,
    ) -> Result<DKGMessage> {
        // In the miniscule chance that the locally aggregated subtranscript is the same as the
        // remote transcript, we can just sign and return immediately.
        if local_aggregated_transcript.hash() == req.subtranscript_hash {
            let signature = ssk
                .sign(local_aggregated_transcript.as_ref())
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
        ensure!(
            req.dealer_transcript_hashes.len() == dealer_addresses.len(),
            "dealer_transcript_hashes length mismatch with dealers"
        );

        // Detect mismatched or missing dealers by comparing per-dealer hashes.
        // A malicious dealer may have equivocated (sent different transcripts to different
        // validators), so we must fetch any dealer whose local transcript hash differs from
        // the requester's hash — not just dealers we're missing entirely.
        //
        // Arc-clone out under brief lock, then hash outside to avoid blocking the main loop.
        let transcript_snapshot: Vec<(AccountAddress, Option<Arc<ChunkyTranscript>>)> = {
            let map = received_transcripts.read();
            dealer_addresses
                .iter()
                .map(|addr| (*addr, map.get(addr).cloned()))
                .collect()
        };

        let mut subtranscripts: Vec<ChunkySubtranscript> = Vec::new();
        let mut mismatched_dealers: Vec<AccountAddress> = Vec::new();

        for (i, (addr, maybe_transcript)) in transcript_snapshot.iter().enumerate() {
            let expected_hash = req.dealer_transcript_hashes[i];
            match maybe_transcript {
                Some(transcript) => {
                    let bytes = bcs::to_bytes(transcript.as_ref())
                        .map_err(|e| anyhow!("transcript serialization error: {e}"))?;
                    let local_hash = aptos_crypto::HashValue::sha3_256_of(&bytes);
                    if local_hash == expected_hash {
                        subtranscripts.push(transcript.get_subtranscript());
                    } else {
                        // Equivocated: local transcript differs from requester's
                        mismatched_dealers.push(*addr);
                    }
                },
                None => {
                    // Missing entirely
                    mismatched_dealers.push(*addr);
                },
            }
        }

        // Fetch mismatched/missing transcripts from the requester specifically.
        // Only the requester knows which transcripts they used — we can't fan out to
        // other peers due to equivocation.
        if !mismatched_dealers.is_empty() {
            let fetcher = TranscriptFetcher::new(
                sender,
                req.dealer_epoch,
                mismatched_dealers,
                Duration::from_secs(10),
                Arc::clone(&dkg_config),
                epoch_state.clone(),
            );

            let fetched_transcripts = fetcher.run(network_sender).await?;

            // Use fetched transcripts ephemerally for re-aggregation (don't store in
            // received_transcripts — equivocation is rare and caching would complicate
            // the map with one dealer → multiple valid transcripts).
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

        // Aggregate subtranscripts in projective form, then normalize (same logic as chunky_agg_trx_producer)
        let recomputed_subtranscript =
            ChunkySubtranscript::aggregate(&dkg_config.threshold_config, subtranscripts)
                .context("failed to aggregate subtranscripts")?;
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

    #[cfg(test)]
    pub(crate) fn new_for_testing(
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
            received_transcripts: Arc::new(RwLock::new(HashMap::new())),
            rpc_handler_guards: HashMap::new(),
            stopped: false,
            state: InnerState::Init,
        }
    }

    #[cfg(test)]
    pub(crate) fn state_name(&self) -> &str {
        self.state.variant_name()
    }

    #[cfg(test)]
    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped
    }
}
