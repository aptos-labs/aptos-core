// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::{
        agg_subtrx_producer,
        missing_transcript_fetcher::TranscriptFetcher,
        subtrx_cert_producer,
        types::{
            AggregatedSubtranscriptWithHashes, CertifiedAggregatedSubtranscript,
            ChunkyTranscriptWithHash, MissingTranscriptRequest, MissingTranscriptResponse,
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
use aptos_crypto::{bls12381, hash::CryptoHash, HashValue, SigningKey, Uniform};
use aptos_dkg::pvss::{
    traits::{transcript::Aggregatable, Transcript},
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
use fail::fail_point;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use rand::{prelude::StdRng, thread_rng, Rng, SeedableRng};
use std::{collections::HashMap, fmt, mem, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tokio_retry::strategy::ExponentialBackoff;

#[cfg(test)]
mod tests;

struct AbortOnDrop(JoinHandle<()>);

impl AbortOnDrop {
    fn is_finished(&self) -> bool {
        self.0.is_finished()
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

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
        _agg_abort_guard: DropGuard,
    },
    Finished {
        vtxn_guard: TxnGuard,
        start_time: Duration,
        my_transcript: Arc<ChunkyDKGTranscript>,
        aggregated_subtranscript: Arc<AggregatedSubtranscript>,
        dkg_config: Arc<ChunkyDKGSession>,
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
    // Values use ChunkyTranscriptWithHash for cached hash lookups.
    received_transcripts: Arc<RwLock<HashMap<AccountAddress, ChunkyTranscriptWithHash>>>,

    // Guards for spawned RPC handler tasks, keyed by requesting validator's address.
    // Tuple of (subtranscript_hash, handle). Skip-if-running: if a handler for the same
    // sender+hash is still running, skip spawning a new one.
    rpc_handler_guards: HashMap<AccountAddress, (HashValue, AbortOnDrop)>,

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

        let session = ChunkyDKGSession::new(dkg_session_metadata);

        let session_clone = session.clone();
        let ssk_clone = self.ssk.clone();
        let spk_clone = self.spk.clone();
        let my_index = self.my_index;

        let trx = tokio::task::spawn_blocking(move || {
            let mut rng = StdRng::from_rng(thread_rng()).unwrap();
            let input_secret = ChunkyInputSecret::generate(&mut rng);
            let dealer = Player { id: my_index };

            monitor!(
                "chunky_dkg_deal_transcript",
                ChunkyTranscript::deal(
                    &session_clone.threshold_config,
                    &session_clone.public_parameters,
                    &ssk_clone,
                    &spk_clone,
                    &session_clone.eks,
                    &input_secret,
                    &session_clone.session_metadata,
                    &dealer,
                    &mut rng,
                )
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
            session.clone(),
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
            dkg_config: session,
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
            _abort_guard: agg_abort_guard,
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
            _agg_abort_guard: agg_abort_guard,
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
            aggregated_subtranscript: local_agg_subtrx,
            dkg_config,
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
        let agg_subtrx_for_blocking = Arc::clone(&aggregated_subtranscript);
        let (encryption_key_bytes, transcript_bytes) = tokio::task::spawn_blocking(move || {
            let digest_key = DIGEST_KEY
                .as_ref()
                .ok_or_else(|| anyhow!("DigestKey not available; cannot derive encryption key"))?;
            let key = agg_subtrx_for_blocking.derive_encryption_key_bytes(digest_key.tau_g2)?;
            let bytes = bcs::to_bytes(agg_subtrx_for_blocking.as_ref())
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
        let _ = local_agg_subtrx;
        self.state = InnerState::Finished {
            vtxn_guard,
            start_time,
            my_transcript,
            aggregated_subtranscript,
            dkg_config,
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
        fail_point!("chunky_dkg::process_dkg_start_event", |_| Ok(()));
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
            .map(|twh| Arc::clone(&twh.transcript));
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
                aggregated_subtranscript,
                dkg_config,
                ..
            }
            | InnerState::Finished {
                aggregated_subtranscript,
                dkg_config,
                ..
            } => (Arc::clone(aggregated_subtranscript), Arc::clone(dkg_config)),
            _ => {
                response_sender.send(Err(anyhow!(
                    "[ChunkyDKG] not ready for signature requests in state {:?}",
                    self.state.variant_name()
                )));
                return Ok(());
            },
        };

        let req_subtranscript_hash = req.subtranscript_hash;

        // Rate limit: at most one concurrent handler per sender. This prevents
        // a malicious validator from spawning many expensive handlers by varying
        // the subtranscript hash. Also deduplicates ReliableBroadcast retries.
        if let Some((_existing_hash, handle)) = self.rpc_handler_guards.get(&sender) {
            if !handle.is_finished() {
                counters::CHUNKY_DKG_SIGNATURE_REQUEST_SKIPPED.inc();
                response_sender.send(Err(anyhow!(
                    "handler already in-flight for sender {}",
                    sender
                )));
                return Ok(());
            }
        }

        // Spawn a tokio task to handle the validation computation.
        let received_transcripts = self.received_transcripts.clone();
        let epoch_state = self.epoch_state.clone();
        let ssk = self.ssk.clone();
        let my_addr = self.my_addr;
        let epoch = self.epoch_state.epoch;
        let network_sender = self.network_sender.clone();
        let handle = tokio::spawn(async move {
            const HANDLER_TIMEOUT: Duration = Duration::from_secs(60);
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
        });
        self.rpc_handler_guards
            .insert(sender, (req_subtranscript_hash, AbortOnDrop(handle)));

        Ok(())
    }

    /// Resolve all subtranscripts required by a signature request: validate the bitmask,
    /// check local storage, poll for late arrivals, and fetch any still-missing transcripts.
    async fn resolve_subtranscripts(
        sender: AccountAddress,
        req: &ChunkyDKGSubtranscriptSignatureRequest,
        received_transcripts: &Arc<RwLock<HashMap<AccountAddress, ChunkyTranscriptWithHash>>>,
        epoch_state: &Arc<EpochState>,
        dkg_config: &Arc<ChunkyDKGSession>,
        network_sender: Arc<NetworkSender>,
    ) -> Result<Vec<ChunkySubtranscript>> {
        let num_validators = epoch_state.verifier.len();
        let ordered_addrs = epoch_state.verifier.get_ordered_account_addresses();
        let max_bit = req
            .dealer_bitmask
            .last_set_bit()
            .ok_or_else(|| anyhow!("dealer_bitmask is empty"))?;
        ensure!(
            (max_bit as usize) < num_validators,
            "dealer_bitmask contains out-of-range bits"
        );
        let num_dealers = req.dealer_bitmask.count_ones() as usize;
        ensure!(
            req.dealer_transcript_hashes.len() == num_dealers,
            "dealer_transcript_hashes length mismatch with dealer_bitmask popcount"
        );

        // Safety: last_set_bit check above guarantees all indices are < num_validators.
        let dealers: Vec<(AccountAddress, HashValue)> = req
            .dealer_bitmask
            .iter_ones()
            .map(|idx| ordered_addrs[idx])
            .zip(req.dealer_transcript_hashes.iter().copied())
            .collect();

        epoch_state
            .verifier
            .check_voting_power(dealers.iter().map(|(addr, _)| addr), true)
            .map_err(|e| anyhow!("dealer set does not meet quorum: {:?}", e))?;

        let check_local = || {
            let map = received_transcripts.read();
            let mut subtranscripts = Vec::new();
            let mut missing = Vec::new();
            for &(addr, expected_hash) in &dealers {
                match map.get(&addr) {
                    Some(twh) if twh.hash() == expected_hash => {
                        subtranscripts.push(twh.get_subtranscript())
                    },
                    _ => missing.push(addr),
                }
            }
            (subtranscripts, missing)
        };

        let (mut subtranscripts, missing_dealers) = check_local();

        if !missing_dealers.is_empty() {
            // Poll received_transcripts to let the aggregator resolve mismatches.
            // Most of the time, the aggregator collects all needed transcripts
            // within this window, eliminating the need to fetch entirely.
            // The first RPC from the requester will time out (RB rpc_timeout_ms = 10s),
            // but skip-if-running keeps this handler alive across retries.
            const MAX_WAIT: Duration = Duration::from_secs(10);
            const POLL_INTERVAL: Duration = Duration::from_millis(500);
            const MAX_FETCH_JITTER: Duration = Duration::from_secs(5);
            let jitter = Duration::from_millis(
                rand::thread_rng().gen_range(0, MAX_FETCH_JITTER.as_millis() as u64),
            );
            let deadline = tokio::time::Instant::now() + MAX_WAIT + jitter;

            let mut still_missing = missing_dealers.clone();
            while tokio::time::Instant::now() < deadline {
                tokio::time::sleep(POLL_INTERVAL).await;
                let (s, m) = check_local();
                subtranscripts = s;
                still_missing = m;
                if still_missing.is_empty() {
                    break;
                }
            }

            let resolved = missing_dealers.len().saturating_sub(still_missing.len());
            info!(
                sender = sender,
                initial_mismatches = missing_dealers.len(),
                resolved_by_delay = resolved,
                still_missing = still_missing.len(),
                "[ChunkyDKG] Post-delay recheck: {}/{} mismatches resolved by aggregator",
                resolved,
                missing_dealers.len(),
            );

            if !still_missing.is_empty() {
                let fetcher = TranscriptFetcher::new(
                    sender,
                    req.dealer_epoch,
                    still_missing,
                    Duration::from_secs(10),
                    Arc::clone(dkg_config),
                    epoch_state.clone(),
                );
                let fetched = monitor!(
                    "chunky_dkg_transcript_fetch",
                    fetcher.run(network_sender).await
                );
                match fetched {
                    Ok(transcripts) => {
                        counters::CHUNKY_DKG_TRANSCRIPT_FETCH_TOTAL
                            .with_label_values(&["success"])
                            .inc();
                        for t in transcripts.into_values() {
                            subtranscripts.push(t.get_subtranscript());
                        }
                    },
                    Err(e) => {
                        counters::CHUNKY_DKG_TRANSCRIPT_FETCH_TOTAL
                            .with_label_values(&["failure"])
                            .inc();
                        return Err(e);
                    },
                }
            }
        }

        ensure!(
            !subtranscripts.is_empty(),
            "No transcripts found for required dealers"
        );
        ensure!(
            subtranscripts.len() == num_dealers,
            "Not enough subtranscripts"
        );

        Ok(subtranscripts)
    }

    /// Aggregate resolved subtranscripts, verify the hash matches the request, and sign.
    /// CPU-heavy work runs inside `spawn_blocking`.
    async fn aggregate_and_sign(
        subtranscripts: Vec<ChunkySubtranscript>,
        req: &ChunkyDKGSubtranscriptSignatureRequest,
        dkg_config: &ChunkyDKGSession,
        ssk: &Arc<DealerPrivateKey>,
    ) -> Result<(AggregatedSubtranscript, bls12381::Signature)> {
        let dealer_epoch = req.dealer_epoch;
        let dealer_bitmask = req.dealer_bitmask.clone();
        let subtranscript_hash = req.subtranscript_hash;
        let threshold_config = dkg_config.threshold_config.clone();
        let ssk = Arc::clone(ssk);
        tokio::task::spawn_blocking(move || {
            let recomputed_subtranscript =
                ChunkySubtranscript::aggregate(&threshold_config, subtranscripts)
                    .context("failed to aggregate subtranscripts")?;
            let recomputed = AggregatedSubtranscript {
                dealer_epoch,
                subtranscript: recomputed_subtranscript,
                dealer_bitmask,
            };
            ensure!(
                recomputed.hash() == subtranscript_hash,
                "subtranscript hash mismatch in validation request"
            );
            let sig = ssk
                .sign(&recomputed)
                .map_err(|e| anyhow!("failed to sign subtranscript validation: {:?}", e))?;
            Ok((recomputed, sig))
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking join error: {e}"))?
    }

    /// Handle subtranscript validation computation.
    async fn handle_subtranscript_signature_request(
        sender: AccountAddress,
        req: ChunkyDKGSubtranscriptSignatureRequest,
        local_aggregated_transcript: Arc<AggregatedSubtranscript>,
        dkg_config: Arc<ChunkyDKGSession>,
        ssk: Arc<DealerPrivateKey>,
        _my_addr: AccountAddress,
        received_transcripts: Arc<RwLock<HashMap<AccountAddress, ChunkyTranscriptWithHash>>>,
        epoch_state: Arc<EpochState>,
        network_sender: Arc<NetworkSender>,
    ) -> Result<DKGMessage> {
        // Fast path: local aggregated transcript matches — sign and return immediately.
        if local_aggregated_transcript.hash() == req.subtranscript_hash {
            let signature = ssk
                .sign(local_aggregated_transcript.as_ref())
                .map_err(|e| anyhow!("failed to sign subtranscript validation: {:?}", e))?;
            return Ok(DKGMessage::SubtranscriptSignatureResponse(
                ChunkyDKGSubtranscriptSignatureResponse::new(
                    req.dealer_epoch,
                    req.subtranscript_hash,
                    signature,
                ),
            ));
        }

        let subtranscripts = Self::resolve_subtranscripts(
            sender,
            &req,
            &received_transcripts,
            &epoch_state,
            &dkg_config,
            network_sender,
        )
        .await?;

        let (_recomputed, signature) =
            Self::aggregate_and_sign(subtranscripts, &req, &dkg_config, &ssk).await?;

        Ok(DKGMessage::SubtranscriptSignatureResponse(
            ChunkyDKGSubtranscriptSignatureResponse::new(
                req.dealer_epoch,
                req.subtranscript_hash,
                signature,
            ),
        ))
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
