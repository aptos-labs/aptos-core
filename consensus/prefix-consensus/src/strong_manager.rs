// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Strong Prefix Consensus Manager
//!
//! Async event loop that orchestrates the multi-view Strong Prefix Consensus
//! protocol. Drives per-view Prefix Consensus instances and feeds their outputs
//! to the Strong Protocol state machine (`StrongPrefixConsensusProtocol`).
//!
//! The manager handles:
//! - View lifecycle (creation, PC execution, completion)
//! - Two view entry triggers: (a) completed previous view, (b) received proposal
//! - Certificate broadcasting and storage
//! - Empty-view handling (EmptyViewMessage collection → IndirectCertificate)
//! - Certificate fetching for trace-back liveness
//! - Commit message construction and broadcast
//! - Slot/epoch filtering for replay prevention

use crate::{
    certificates::{
        Certificate, EmptyViewMessage, EmptyViewStatement,
        IndirectCertificate, StrongPCCommit,
    },
    inner_pc_impl::ThreeRoundPC,
    inner_pc_trait::InnerPCAlgorithm,
    network_interface::SubprotocolNetworkSender,
    network_messages::{PrefixConsensusMsg, StrongPrefixConsensusMsg},
    strong_protocol::{
        ChainBuildError, StrongPrefixConsensusProtocol, View1Decision, ViewDecision,
    },
    types::{
        CertFetchRequest, CertFetchResponse, PartyId, PrefixConsensusInput, PrefixConsensusOutput,
        PrefixVector, ViewProposal, QC3,
    },
    view_state::{RankingManager, ViewOutput, ViewState},
};
use anyhow::Result;
use aptos_consensus_types::common::Author;
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use futures::{FutureExt, StreamExt};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::time::Sleep;

/// How long to wait for the first-ranked certificate before starting the inner PC
/// with whatever certificates are available (possibly none).
///
/// TODO: Make this configurable (e.g., via consensus config or constructor parameter).
const VIEW_START_TIMEOUT: Duration = Duration::from_millis(300);

// ============================================================================
// Per-View Inner PC State
// ============================================================================

/// State for a per-view Prefix Consensus instance
struct PCState<T: InnerPCAlgorithm> {
    /// The inner PC algorithm instance
    algorithm: T,
    /// Whether this view's PC has started (Round 1 broadcast)
    started: bool,
    /// Whether this view's PC has completed (QC3 formed)
    completed: bool,
}

// ============================================================================
// Strong Manager
// ============================================================================

/// Type alias for the default Strong PC Manager using the 3-round protocol.
pub type DefaultStrongPCManager<NS> = StrongPrefixConsensusManager<NS, ThreeRoundPC>;

/// Manager for the multi-view Strong Prefix Consensus protocol.
///
/// Generic over the inner PC algorithm `T`, allowing different implementations
/// (e.g., 3-round standard, 2-round good-case from Appendix D) to be swapped in
/// without changing the view management, proposal, commit, or event loop logic.
pub struct StrongPrefixConsensusManager<NetworkSender, T: InnerPCAlgorithm> {
    // Identity
    party_id: PartyId,
    epoch: u64,
    slot: u64,

    // Strong Protocol state machine (Chunk 1)
    protocol: StrongPrefixConsensusProtocol,

    // Per-view state
    ranking_manager: RankingManager,
    current_view: u64,
    view_states: HashMap<u64, ViewState>,
    pc_states: HashMap<u64, PCState<T>>,

    // Duplicate detection for proposals and empty-view messages
    seen_proposals: HashMap<u64, HashSet<PartyId>>,
    seen_empty_views: HashMap<u64, HashSet<PartyId>>,

    // Empty-view collection (for IndirectCertificate creation)
    empty_view_collectors: HashMap<u64, Vec<EmptyViewMessage>>,

    // Pending commit: if we need to trace-back but are missing certs
    pending_commit_proof: Option<QC3>,

    // Pending fetches (hash → number of attempts)
    pending_fetches: HashMap<HashValue, u32>,

    // Per-view start timer: fires after VIEW_START_TIMEOUT to start the inner PC
    // even if no first-ranked certificate has arrived.
    view_start_timer: Option<(u64, std::pin::Pin<Box<Sleep>>)>,

    // Network and signing
    network_sender: NetworkSender,
    validator_signer: ValidatorSigner,
    validator_verifier: Arc<ValidatorVerifier>,

    // Input vector for View 1
    input_vector: PrefixVector,
}

impl<NetworkSender: SubprotocolNetworkSender<StrongPrefixConsensusMsg>, T: InnerPCAlgorithm<Message = PrefixConsensusMsg>>
    StrongPrefixConsensusManager<NetworkSender, T>
{
    /// Create a new Strong Prefix Consensus manager
    pub fn new(
        party_id: PartyId,
        epoch: u64,
        slot: u64,
        initial_ranking: Vec<PartyId>,
        input_vector: PrefixVector,
        network_sender: NetworkSender,
        validator_signer: ValidatorSigner,
        validator_verifier: Arc<ValidatorVerifier>,
    ) -> Self {
        Self {
            party_id,
            epoch,
            slot,
            protocol: StrongPrefixConsensusProtocol::new(epoch, slot),
            ranking_manager: RankingManager::new(initial_ranking),
            current_view: 0, // Not yet started
            view_states: HashMap::new(),
            pc_states: HashMap::new(),
            seen_proposals: HashMap::new(),
            seen_empty_views: HashMap::new(),
            empty_view_collectors: HashMap::new(),
            pending_commit_proof: None,
            pending_fetches: HashMap::new(),
            view_start_timer: None,
            network_sender,
            validator_signer,
            validator_verifier,
            input_vector,
        }
    }

    // ========================================================================
    // Event Loop
    // ========================================================================

    /// Main event loop
    ///
    /// Runs until the protocol completes (all parties agree on v_high) or
    /// a shutdown signal is received.
    pub async fn run(
        mut self,
        mut message_rx: aptos_channels::UnboundedReceiver<(Author, StrongPrefixConsensusMsg)>,
        close_rx: futures::channel::oneshot::Receiver<futures::channel::oneshot::Sender<()>>,
    ) {
        info!(
            party_id = %self.party_id,
            epoch = self.epoch,
            slot = self.slot,
            "StrongPrefixConsensusManager event loop started"
        );

        // Start View 1
        if let Err(e) = self.start_view1().await {
            error!(
                party_id = %self.party_id,
                error = ?e,
                "Failed to start View 1"
            );
            return;
        }

        let mut close_rx = close_rx.into_stream();

        loop {
            tokio::select! {
                biased;

                close_req = close_rx.select_next_some() => {
                    info!(
                        party_id = %self.party_id,
                        "Received shutdown signal"
                    );
                    if let Ok(ack_sender) = close_req {
                        let _ = ack_sender.send(());
                    }
                    break;
                }

                // View start timer: fires when we've waited long enough for first-ranked cert
                _ = async {
                    if let Some((_view, timer)) = &mut self.view_start_timer {
                        timer.as_mut().await;
                    } else {
                        futures::future::pending::<()>().await;
                    }
                } => {
                    if let Some((view, _)) = self.view_start_timer.take() {
                        info!(
                            party_id = %self.party_id,
                            view = view,
                            "View start timer expired, starting inner PC with available certificates"
                        );
                        self.start_pc_now(view).await;
                    }
                }

                Some((author, msg)) = message_rx.next() => {
                    self.process_message(author, msg).await;

                    if self.protocol.is_complete() {
                        info!(
                            party_id = %self.party_id,
                            "Strong Prefix Consensus complete"
                        );
                        break;
                    }
                }
            }
        }

        info!(
            party_id = %self.party_id,
            "StrongPrefixConsensusManager event loop terminated"
        );
    }

    // ========================================================================
    // View 1 Lifecycle
    // ========================================================================

    /// Start View 1 with the raw input vector
    async fn start_view1(&mut self) -> Result<()> {
        self.current_view = 1;

        info!(
            party_id = %self.party_id,
            input_len = self.input_vector.len(),
            "Starting View 1"
        );

        // Create inner PC algorithm for View 1
        let input = PrefixConsensusInput::new(
            self.input_vector.clone(),
            self.party_id,
            self.epoch,
            self.slot,
            1, // view 1
        );
        let mut algorithm = T::new_for_view(input, self.validator_verifier.clone());
        let (outbound_msgs, output) = algorithm.start(&self.validator_signer).await?;

        let pc_state = PCState {
            algorithm,
            started: true,
            completed: false,
        };
        self.pc_states.insert(1, pc_state);

        // Broadcast all outbound messages (may include Vote1, Vote2, Vote3 if cascading)
        for out_msg in outbound_msgs {
            let msg = StrongPrefixConsensusMsg::InnerPC { view: 1, msg: out_msg };
            self.network_sender.broadcast(msg).await;
        }

        // If the inner PC completed during start (all rounds cascaded)
        if let Some(pc_output) = output {
            self.finalize_view(1, pc_output).await;
        }

        Ok(())
    }

    /// Handle View 1 completion
    async fn handle_view1_complete(&mut self, output: ViewOutput) {
        let decision = self.protocol.process_view1_output(output);

        match decision {
            View1Decision::DirectCert(cert) => {
                let cert_enum = Certificate::Direct(cert);
                let cert_hash = cert_enum.hash();
                self.protocol.store_certificate(cert_enum.clone());

                info!(
                    party_id = %self.party_id,
                    cert_hash = %cert_hash,
                    "View 1 complete, broadcasting proposal for View 2"
                );

                self.propose_and_enter(2, cert_enum).await;
            }
        }
    }

    // ========================================================================
    // View W > 1 Lifecycle
    // ========================================================================

    /// Broadcast a proposal for `next_view` with the given certificate, add our own
    /// certificate to the next view's ViewState, and enter the view.
    /// Broadcast a proposal for `next_view` with the given certificate, add it to
    /// the ViewState, and enter the view.
    ///
    /// If we've already entered `next_view` (or a later view), this is a no-op.
    /// This prevents broadcasting two different proposals for the same view, which
    /// would be Byzantine behavior and could cause prefix cuts in the inner PC.
    async fn propose_and_enter(&mut self, next_view: u64, cert: Certificate) {
        if next_view <= self.current_view {
            // Already at or past this view — we've already proposed. The cert
            // is stored in the cert store by the caller; skip broadcast and enter.
            return;
        }

        // Broadcast proposal to other parties
        let proposal = ViewProposal::new(next_view, cert.clone(), self.epoch, self.slot);
        let msg = StrongPrefixConsensusMsg::Proposal(Box::new(proposal));
        self.network_sender.broadcast(msg).await;

        // Add our own certificate to the next view's ViewState so it's available
        // when enter_view checks for the first-ranked cert.
        if !self.view_states.contains_key(&next_view) {
            let ranking = self.ranking_manager.get_ranking_for_view(next_view);
            self.view_states
                .insert(next_view, ViewState::new(next_view, self.slot, ranking));
        }
        self.view_states
            .get_mut(&next_view)
            .unwrap()
            .add_certificate(self.party_id, cert);

        self.enter_view(next_view).await;
    }

    /// Enter a new view (either from completing previous or receiving a proposal)
    ///
    /// Starts the inner PC immediately if the first-ranked certificate is already
    /// available. Otherwise, sets a timer (VIEW_START_TIMEOUT) and starts when
    /// either the first-ranked cert arrives or the timer expires.
    async fn enter_view(&mut self, view: u64) {
        if view <= self.current_view {
            return; // Already at or past this view
        }

        self.current_view = view;

        info!(
            party_id = %self.party_id,
            view = view,
            "Entering View {}", view
        );

        // Create ViewState if not exists
        if !self.view_states.contains_key(&view) {
            let ranking = self.ranking_manager.get_ranking_for_view(view);
            self.view_states.insert(view, ViewState::new(view, self.slot, ranking));
        }

        if self.has_first_ranked_cert(view) {
            // Best case: start immediately with the optimal (shortest) input vector
            self.start_pc_now(view).await;
        } else {
            // Set timer: start when first-ranked cert arrives or timeout expires
            self.view_start_timer = Some((
                view,
                Box::pin(tokio::time::sleep(VIEW_START_TIMEOUT)),
            ));
        }
    }

    /// Check if the first-ranked party's certificate is available for a view.
    fn has_first_ranked_cert(&self, view: u64) -> bool {
        self.view_states
            .get(&view)
            .map_or(false, |vs| vs.get_first_certificate_position() == Some(0))
    }

    /// Try to start the inner PC for the current view if the first-ranked
    /// certificate just arrived. Cancels the pending timer if so.
    ///
    /// Called from `process_proposal` when a certificate is added to a view
    /// we've already entered.
    async fn try_start_pc(&mut self, view: u64) {
        // Already started?
        if self.pc_states.get(&view).map_or(false, |s| s.started) {
            return;
        }

        if !self.has_first_ranked_cert(view) {
            return; // Wait for timer or first-ranked cert
        }

        // Cancel the timer if it's for this view
        if matches!(self.view_start_timer, Some((v, _)) if v == view) {
            self.view_start_timer = None;
        }

        self.start_pc_now(view).await;
    }

    /// Unconditionally start the inner PC for a view with whatever certificates
    /// are currently available (possibly none, resulting in an all-bot input).
    ///
    /// TODO: Consider sending an EmptyViewMessage directly (without running the
    /// inner PC) when no certificates have arrived at timeout. This would save
    /// 3 rounds of inner PC for the all-bot case. However, all parties must still
    /// participate in the inner PC to ensure other parties can form QCs — so this
    /// optimization only applies when ALL parties have no certificates for the view.
    fn start_pc_now(&mut self, view: u64) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
        // Already started?
        if self.pc_states.get(&view).map_or(false, |s| s.started) {
            return;
        }

        let view_state = self
            .view_states
            .get(&view)
            .expect("ViewState must exist when starting PC");

        // Build truncated input vector from received certificates
        let input_vector = view_state.build_truncated_input_vector();

        let has_certs = input_vector.iter().any(|h| *h != HashValue::zero());
        info!(
            party_id = %self.party_id,
            view = view,
            input_len = input_vector.len(),
            has_certs = has_certs,
            "Starting inner PC for View {}", view
        );

        // Create inner PC algorithm
        let input = PrefixConsensusInput::new(
            input_vector,
            self.party_id,
            self.epoch,
            self.slot,
            view,
        );
        let mut algorithm = T::new_for_view(input, self.validator_verifier.clone());

        match algorithm.start(&self.validator_signer).await {
            Ok((outbound_msgs, output)) => {
                let pc_state = PCState {
                    algorithm,
                    started: true,
                    completed: false,
                };
                self.pc_states.insert(view, pc_state);

                for out_msg in outbound_msgs {
                    let msg = StrongPrefixConsensusMsg::InnerPC { view, msg: out_msg };
                    self.network_sender.broadcast(msg).await;
                }

                if let Some(pc_output) = output {
                    self.finalize_view(view, pc_output).await;
                }
            }
            Err(e) => {
                error!(
                    party_id = %self.party_id,
                    view = view,
                    error = ?e,
                    "Failed to start inner PC for view"
                );
            }
        }
        })
    }

    /// Mark a view's inner PC as complete and dispatch to the appropriate handler.
    async fn finalize_view(&mut self, view: u64, pc_output: PrefixConsensusOutput) {
        self.pc_states
            .get_mut(&view)
            .expect("PCState must exist when finalizing view")
            .completed = true;

        let output = ViewOutput {
            view,
            slot: self.slot,
            v_low: pc_output.v_low,
            v_high: pc_output.v_high,
            proof: pc_output.qc3,
        };

        if view == 1 {
            self.handle_view1_complete(output).await;
        } else {
            self.handle_view_complete(view, output).await;
        }
    }

    /// Handle view W > 1 completion
    async fn handle_view_complete(&mut self, view: u64, output: ViewOutput) {
        let decision = self.protocol.process_view_output(output);

        match decision {
            ViewDecision::Commit { committing_proof } => {
                self.handle_commit_decision(committing_proof).await;
            }
            ViewDecision::DirectCert(cert) => {
                let cert_enum = Certificate::Direct(cert);
                let cert_hash = cert_enum.hash();
                self.protocol.store_certificate(cert_enum.clone());

                info!(
                    party_id = %self.party_id,
                    view = view,
                    cert_hash = %cert_hash,
                    "DirectCert from View {}, broadcasting proposal for View {}",
                    view, view + 1
                );

                self.propose_and_enter(view + 1, cert_enum).await;
            }
            ViewDecision::EmptyView => {
                info!(
                    party_id = %self.party_id,
                    view = view,
                    "EmptyView from View {}, broadcasting EmptyViewMessage", view
                );
                self.handle_empty_view_decision(view).await;
            }
        }
    }

    /// Handle commit decision: try to build chain, broadcast commit or fetch
    async fn handle_commit_decision(&mut self, committing_proof: QC3) {
        match self.protocol.build_commit_message(&committing_proof) {
            Ok(commit) => {
                info!(
                    party_id = %self.party_id,
                    "Built StrongPCCommit, broadcasting"
                );
                let v_high = commit.v_high.clone();
                let msg = StrongPrefixConsensusMsg::Commit(Box::new(commit));
                self.network_sender.broadcast(msg).await;
                self.protocol.set_committed(v_high);
                self.write_output_file();
            }
            Err(ChainBuildError::MissingCert { hash }) => {
                info!(
                    party_id = %self.party_id,
                    missing_hash = %hash,
                    "Missing cert for trace-back, initiating fetch"
                );
                self.pending_commit_proof = Some(committing_proof);
                self.initiate_fetch(hash).await;
            }
            Err(e) => {
                error!(
                    party_id = %self.party_id,
                    error = %e,
                    "Failed to build commit message"
                );
            }
        }
    }

    /// Handle empty view: sign and broadcast EmptyViewMessage
    async fn handle_empty_view_decision(&mut self, view: u64) {
        let hk = match self.protocol.highest_known_view() {
            Some(hk) => hk.clone(),
            None => {
                error!(
                    party_id = %self.party_id,
                    view = view,
                    "No highest known view for EmptyViewMessage"
                );
                return;
            }
        };

        // Create and sign the statement
        let statement = EmptyViewStatement::new(view, hk.view);
        let signature = self
            .validator_signer
            .sign(&statement)
            .expect("Signing should succeed");

        let empty_msg = EmptyViewMessage::new(
            view,
            self.party_id,
            hk.view,
            hk.proof.clone(),
            signature,
            self.epoch,
            self.slot,
        );

        // Broadcast
        let msg = StrongPrefixConsensusMsg::EmptyView(Box::new(empty_msg.clone()));
        self.network_sender.broadcast(msg).await;

        // Add our own to the collector
        self.empty_view_collectors
            .entry(view)
            .or_default()
            .push(empty_msg);

        // Check if we already have enough (unlikely with just our own)
        self.try_form_indirect_cert(view).await;
    }

    /// Try to form IndirectCertificate from collected EmptyViewMessages
    async fn try_form_indirect_cert(&mut self, view: u64) {
        let messages = self
            .empty_view_collectors
            .get(&view)
            .expect("empty_view_collectors must have entry when forming indirect cert")
            .clone();

        // Check if we have >1/3 stake
        if let Some(indirect_cert) =
            IndirectCertificate::from_messages(view, messages, &self.validator_verifier)
        {
            let cert = Certificate::Indirect(indirect_cert);
            let cert_hash = cert.hash();
            self.protocol.store_certificate(cert.clone());

            info!(
                party_id = %self.party_id,
                view = view,
                cert_hash = %cert_hash,
                "IndirectCert formed for View {}, broadcasting proposal for View {}",
                view, view + 1
            );

            self.propose_and_enter(view + 1, cert).await;
        }
    }

    // ========================================================================
    // Message Routing
    // ========================================================================

    /// Process an incoming network message
    async fn process_message(&mut self, author: Author, msg: StrongPrefixConsensusMsg) {
        // Slot/epoch filtering (prevents cross-slot replays)
        if msg.epoch() != self.epoch {
            debug!(
                party_id = %self.party_id,
                msg_epoch = msg.epoch(),
                expected_epoch = self.epoch,
                "Ignoring message from wrong epoch"
            );
            return;
        }
        if msg.slot() != self.slot {
            debug!(
                party_id = %self.party_id,
                msg_slot = msg.slot(),
                expected_slot = self.slot,
                "Ignoring message from wrong slot"
            );
            return;
        }

        match msg {
            StrongPrefixConsensusMsg::InnerPC { view, msg } => {
                self.process_inner_pc(author, view, msg).await;
            }
            StrongPrefixConsensusMsg::Proposal(proposal) => {
                self.process_proposal(author, *proposal).await;
            }
            StrongPrefixConsensusMsg::EmptyView(empty) => {
                self.process_empty_view(author, *empty).await;
            }
            StrongPrefixConsensusMsg::Commit(commit) => {
                self.process_commit(*commit).await;
            }
            StrongPrefixConsensusMsg::FetchRequest(req) => {
                self.process_fetch_request(author, req).await;
            }
            StrongPrefixConsensusMsg::FetchResponse(resp) => {
                self.process_fetch_response(*resp).await;
            }
        }
    }

    // ========================================================================
    // Inner PC Message Handling
    // ========================================================================

    /// Route an inner PC message to the correct view's algorithm.
    ///
    /// The algorithm handles author mismatch checks, signature verification,
    /// vote processing, and round transitions internally.
    async fn process_inner_pc(&mut self, author: Author, view: u64, msg: PrefixConsensusMsg) {
        let pc_state = match self.pc_states.get_mut(&view) {
            Some(state) => state,
            None => {
                debug!(
                    party_id = %self.party_id,
                    view = view,
                    "Ignoring InnerPC for unknown view"
                );
                return;
            }
        };

        if pc_state.completed {
            return;
        }

        match pc_state.algorithm.process_message(author, msg, &self.validator_signer).await {
            Ok((outbound_msgs, output)) => {
                for out_msg in outbound_msgs {
                    let wrapped = StrongPrefixConsensusMsg::InnerPC { view, msg: out_msg };
                    self.network_sender.broadcast(wrapped).await;
                }
                if let Some(pc_output) = output {
                    self.finalize_view(view, pc_output).await;
                }
            }
            Err(e) => {
                warn!(
                    party_id = %self.party_id,
                    view = view,
                    error = ?e,
                    "Inner PC process_message error"
                );
            }
        }
    }

    // ========================================================================
    // Proposal Handling
    // ========================================================================

    /// Process a ViewProposal from another party
    async fn process_proposal(&mut self, author: Author, proposal: ViewProposal) {
        let target_view = proposal.target_view;

        // Validate target_view is reasonable
        if target_view <= 1 {
            return; // View 1 doesn't use proposals
        }

        // Duplicate check
        let seen = self.seen_proposals.entry(target_view).or_default();
        if seen.contains(&author) {
            return;
        }
        seen.insert(author);

        // No signature on ViewProposal itself is needed. The network layer authenticates
        // the sender (author), which determines the ranking position for this certificate.
        // The certificate inside is cryptographically validated below. Any party (honest or
        // Byzantine) can propose any valid certificate — e.g., an honest party may forward
        // a certificate it received from another party to catch up to the current view.
        // What matters is: (1) the certificate is valid, (2) the sender is authenticated.

        // Validate certificate
        if let Err(e) = proposal.certificate.validate(&self.validator_verifier) {
            warn!(
                party_id = %self.party_id,
                author = %author,
                target_view = target_view,
                error = ?e,
                "Invalid certificate in proposal"
            );
            return;
        }

        // Store certificate in protocol's cert store
        self.protocol.store_certificate(proposal.certificate.clone());

        // Update highest known view from the certificate's proof
        if let Certificate::Direct(ref dc) = proposal.certificate {
            self.protocol
                .update_highest_known_view(dc.view(), dc.proof.clone());
        }

        if target_view > self.current_view {
            // We haven't proposed for this view yet. Adopt the received certificate
            // as our own proposal — broadcast it and enter the view. This improves
            // liveness: parties catch up faster

            // First, ensure ViewState exists and add the sender's certificate under
            // their ranking position (propose_and_enter only adds under our position).
            if !self.view_states.contains_key(&target_view) {
                let ranking = self.ranking_manager.get_ranking_for_view(target_view);
                self.view_states
                    .insert(target_view, ViewState::new(target_view, self.slot, ranking));
            }
            self.view_states
                .get_mut(&target_view)
                .unwrap()
                .add_certificate(author, proposal.certificate.clone());

            // Adopt and enter: broadcasts as our proposal, adds under our position, enters view.
            self.propose_and_enter(target_view, proposal.certificate).await;
        } else if target_view == self.current_view {
            // Already in this view (we've already proposed). Just add the certificate
            // to ViewState under the sender's ranking position and try to start the
            // inner PC if the first-ranked cert just arrived.
            if !self.view_states.contains_key(&target_view) {
                let ranking = self.ranking_manager.get_ranking_for_view(target_view);
                self.view_states
                    .insert(target_view, ViewState::new(target_view, self.slot, ranking));
            }
            let view_state = self.view_states.get_mut(&target_view).unwrap();
            view_state.add_certificate(author, proposal.certificate);

            self.try_start_pc(target_view).await;
        }
        // target_view < current_view: stale proposal, ignore
    }

    // ========================================================================
    // Empty View Message Handling
    // ========================================================================

    /// Process an EmptyViewMessage from another party
    async fn process_empty_view(&mut self, author: Author, msg: EmptyViewMessage) {
        let view = msg.empty_view();

        // Author mismatch check
        if msg.author != author {
            return;
        }

        // Duplicate check
        let seen = self.seen_empty_views.entry(view).or_default();
        if seen.contains(&author) {
            return;
        }
        seen.insert(author);

        // TODO: Verify EmptyViewMessage signature

        // Add to collector
        self.empty_view_collectors
            .entry(view)
            .or_default()
            .push(msg);

        // Try to form IndirectCertificate
        self.try_form_indirect_cert(view).await;
    }

    // ========================================================================
    // Commit Handling
    // ========================================================================

    /// Process a StrongPCCommit from another party
    async fn process_commit(&mut self, commit: StrongPCCommit) {
        if self.protocol.is_complete() {
            return; // Already done
        }

        match self
            .protocol
            .process_received_commit(&commit, &self.validator_verifier)
        {
            Ok(()) => {
                info!(
                    party_id = %self.party_id,
                    "Received valid StrongPCCommit, protocol complete"
                );
                self.write_output_file();
            }
            Err(e) => {
                warn!(
                    party_id = %self.party_id,
                    error = %e,
                    "Invalid StrongPCCommit received"
                );
            }
        }
    }

    // ========================================================================
    // Certificate Fetching
    // ========================================================================

    /// Initiate a certificate fetch
    async fn initiate_fetch(&mut self, hash: HashValue) {
        let attempts = self.pending_fetches.entry(hash).or_insert(0);
        *attempts += 1;

        if *attempts > 10 {
            warn!(
                party_id = %self.party_id,
                hash = %hash,
                attempts = *attempts,
                "Exceeded fetch attempts, waiting for StrongPCCommit from others"
            );
            return;
        }

        let req = CertFetchRequest::new(hash, self.epoch, self.slot);
        let msg = StrongPrefixConsensusMsg::FetchRequest(req);
        self.network_sender.broadcast(msg).await;
    }

    /// Process a fetch request from another party
    async fn process_fetch_request(&mut self, author: Author, req: CertFetchRequest) {
        if let Some(cert) = self.protocol.get_certificate(&req.cert_hash) {
            let resp = CertFetchResponse::new(
                req.cert_hash,
                cert.clone(),
                self.epoch,
                self.slot,
            );
            let msg = StrongPrefixConsensusMsg::FetchResponse(Box::new(resp));
            self.network_sender.send_to(author, msg).await;
        }
        // If we don't have it, don't reply (per design decision)
    }

    /// Process a fetch response
    async fn process_fetch_response(&mut self, resp: CertFetchResponse) {
        // Ignore unsolicited responses to prevent DoS via expensive validation work
        if !self.pending_fetches.contains_key(&resp.cert_hash) {
            return;
        }

        // Verify hash matches
        if resp.certificate.hash() != resp.cert_hash {
            warn!(
                party_id = %self.party_id,
                expected = %resp.cert_hash,
                got = %resp.certificate.hash(),
                "Cert hash mismatch in fetch response"
            );
            return;
        }

        // Validate certificate
        if let Err(e) = resp.certificate.validate(&self.validator_verifier) {
            warn!(
                party_id = %self.party_id,
                error = ?e,
                "Invalid certificate in fetch response"
            );
            return;
        }

        // Store it and clear the pending fetch
        self.protocol.store_certificate(resp.certificate);
        self.pending_fetches.remove(&resp.cert_hash);

        // Retry pending commit if we have one
        if let Some(proof) = self.pending_commit_proof.clone() {
            self.handle_commit_decision(proof).await;
        }
    }

    // ========================================================================
    // Output Queries
    // ========================================================================

    /// Get the Strong PC v_low output
    pub fn v_low(&self) -> Option<&PrefixVector> {
        self.protocol.v_low()
    }

    /// Get the Strong PC v_high output
    pub fn v_high(&self) -> Option<&PrefixVector> {
        self.protocol.v_high()
    }

    /// Check if the protocol is complete
    pub fn is_complete(&self) -> bool {
        self.protocol.is_complete()
    }

    /// Get this party's ID
    pub fn party_id(&self) -> PartyId {
        self.party_id
    }

    /// Get the epoch
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Get the slot
    pub fn slot(&self) -> u64 {
        self.slot
    }

    /// Write output to file for smoke test validation
    fn write_output_file(&self) {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct OutputFile {
            party_id: String,
            epoch: u64,
            slot: u64,
            input: Vec<String>,
            v_low: Vec<String>,
            v_high: Vec<String>,
        }

        let v_low = match self.protocol.v_low() {
            Some(v) => v.iter().map(|h| h.to_hex()).collect(),
            None => {
                warn!(party_id = %self.party_id, "Cannot write output file: v_low not set");
                return;
            }
        };
        let v_high = match self.protocol.v_high() {
            Some(v) => v.iter().map(|h| h.to_hex()).collect(),
            None => {
                warn!(party_id = %self.party_id, "Cannot write output file: v_high not set");
                return;
            }
        };

        let output_file = OutputFile {
            party_id: format!("{:x}", self.party_id),
            epoch: self.epoch,
            slot: self.slot,
            input: self.input_vector.iter().map(|h| h.to_hex()).collect(),
            v_low,
            v_high,
        };

        let file_path = format!("/tmp/strong_prefix_consensus_output_{:x}.json", self.party_id);
        match serde_json::to_string_pretty(&output_file) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&file_path, json) {
                    warn!(
                        party_id = %self.party_id,
                        error = ?e,
                        "Failed to write strong PC output file"
                    );
                } else {
                    info!(
                        party_id = %self.party_id,
                        file_path = %file_path,
                        "Wrote strong prefix consensus output file"
                    );
                }
            }
            Err(e) => {
                warn!(
                    party_id = %self.party_id,
                    error = ?e,
                    "Failed to serialize strong PC output"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificates::DirectCertificate;
    use aptos_crypto::HashValue;
    use aptos_types::{
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };

    // Mock network sender for testing
    #[derive(Clone)]
    struct MockNetworkSender {
        /// Collect all broadcast messages for inspection
        sent: Arc<tokio::sync::Mutex<Vec<StrongPrefixConsensusMsg>>>,
    }

    impl MockNetworkSender {
        fn new() -> Self {
            Self {
                sent: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }

        async fn sent_messages(&self) -> Vec<StrongPrefixConsensusMsg> {
            self.sent.lock().await.clone()
        }
    }

    #[async_trait::async_trait]
    impl SubprotocolNetworkSender<StrongPrefixConsensusMsg> for MockNetworkSender {
        async fn broadcast(&self, msg: StrongPrefixConsensusMsg) {
            self.sent.lock().await.push(msg);
        }

        async fn send_to(&self, _peer: Author, msg: StrongPrefixConsensusMsg) {
            self.sent.lock().await.push(msg);
        }
    }

    fn create_test_validators(count: usize) -> (Vec<ValidatorSigner>, Arc<ValidatorVerifier>) {
        let signers: Vec<_> = (0..count)
            .map(|_| ValidatorSigner::random(None))
            .collect();

        let validator_infos: Vec<_> = signers
            .iter()
            .map(|signer| {
                ValidatorConsensusInfo::new(
                    signer.author(),
                    signer.public_key(),
                    1, // voting power
                )
            })
            .collect();

        let verifier = Arc::new(ValidatorVerifier::new(validator_infos));
        (signers, verifier)
    }

    fn create_test_manager(
        signers: &mut Vec<ValidatorSigner>,
        verifier: Arc<ValidatorVerifier>,
    ) -> (DefaultStrongPCManager<MockNetworkSender>, MockNetworkSender) {
        let party_id = signers[0].author();
        let initial_ranking: Vec<PartyId> = signers.iter().map(|s| s.author()).collect();
        let network = MockNetworkSender::new();
        let manager = StrongPrefixConsensusManager::new(
            party_id,
            1, // epoch
            0, // slot
            initial_ranking,
            vec![HashValue::random(), HashValue::random()],
            network.clone(),
            signers.remove(0),
            verifier,
        );
        (manager, network)
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let (mut signers, verifier) = create_test_validators(4);
        let (manager, _network) = create_test_manager(&mut signers, verifier);

        assert_eq!(manager.epoch(), 1);
        assert_eq!(manager.slot(), 0);
        assert!(!manager.is_complete());
        assert!(manager.v_low().is_none());
        assert!(manager.v_high().is_none());
    }

    #[tokio::test]
    async fn test_start_view1() {
        let (mut signers, verifier) = create_test_validators(4);
        let (mut manager, network) = create_test_manager(&mut signers, verifier);

        manager.start_view1().await.expect("View 1 should start");

        assert_eq!(manager.current_view, 1);
        assert!(manager.pc_states.contains_key(&1));
        assert!(manager.pc_states[&1].started);

        // Should have broadcast an InnerPC Vote1
        let msgs = network.sent_messages().await;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].name(), "InnerPC");
        assert_eq!(msgs[0].view(), Some(1));
    }

    #[tokio::test]
    async fn test_epoch_filtering() {
        let (mut signers, verifier) = create_test_validators(4);
        let (mut manager, network) = create_test_manager(&mut signers, verifier);
        manager.start_view1().await.unwrap();

        // Clear sent messages from start_view1
        network.sent.lock().await.clear();

        // Send a message with wrong epoch
        let wrong_epoch_proposal = ViewProposal::new(
            2,
            Certificate::Direct(DirectCertificate::new(1, QC3::new(vec![]))),
            999, // wrong epoch
            0,
        );
        let msg = StrongPrefixConsensusMsg::Proposal(Box::new(wrong_epoch_proposal));
        manager.process_message(signers[0].author(), msg).await;

        // Should have been filtered — no new messages sent
        assert!(network.sent_messages().await.is_empty());
    }

    #[tokio::test]
    async fn test_slot_filtering() {
        let (mut signers, verifier) = create_test_validators(4);
        let (mut manager, network) = create_test_manager(&mut signers, verifier);
        manager.start_view1().await.unwrap();
        network.sent.lock().await.clear();

        // Send a message with wrong slot
        let wrong_slot_proposal = ViewProposal::new(
            2,
            Certificate::Direct(DirectCertificate::new(1, QC3::new(vec![]))),
            1, // correct epoch
            5, // wrong slot
        );
        let msg = StrongPrefixConsensusMsg::Proposal(Box::new(wrong_slot_proposal));
        manager.process_message(signers[0].author(), msg).await;

        // Should have been filtered
        assert!(network.sent_messages().await.is_empty());
    }

    #[tokio::test]
    async fn test_process_commit() {
        let (mut signers, verifier) = create_test_validators(4);
        let (mut manager, _network) = create_test_manager(&mut signers, verifier.clone());
        manager.start_view1().await.unwrap();

        // Process a valid commit (this will fail validation since the QC3 is empty,
        // but it tests the routing)
        let commit = StrongPCCommit::new(
            QC3::new(vec![]),
            vec![Certificate::Direct(DirectCertificate::new(
                1,
                QC3::new(vec![]),
            ))],
            vec![HashValue::random()],
            1,
            0,
        );
        manager
            .process_commit(commit)
            .await;

        // Invalid commit should be rejected (not complete)
        assert!(!manager.is_complete());
    }

    #[tokio::test]
    async fn test_proposal_duplicate_rejection() {
        let (mut signers, verifier) = create_test_validators(4);
        let (mut manager, _network) = create_test_manager(&mut signers, verifier);
        manager.start_view1().await.unwrap();

        let author = signers[0].author();
        let cert = Certificate::Direct(DirectCertificate::new(1, QC3::new(vec![])));
        let proposal = ViewProposal::new(2, cert.clone(), 1, 0);

        // First proposal should be accepted
        manager
            .process_proposal(author, proposal.clone())
            .await;
        assert!(manager.seen_proposals[&2].contains(&author));

        // Second proposal from same author for same view should be rejected
        let seen_before = manager.seen_proposals[&2].len();
        manager.process_proposal(author, proposal).await;
        assert_eq!(manager.seen_proposals[&2].len(), seen_before);
    }

    #[tokio::test]
    async fn test_fetch_request_response() {
        let (mut signers, verifier) = create_test_validators(4);
        let (mut manager, network) = create_test_manager(&mut signers, verifier);
        manager.start_view1().await.unwrap();

        // Store a certificate
        let cert = Certificate::Direct(DirectCertificate::new(1, QC3::new(vec![])));
        let cert_hash = cert.hash();
        manager.protocol.store_certificate(cert);

        network.sent.lock().await.clear();

        // Process fetch request
        let req = CertFetchRequest::new(cert_hash, 1, 0);
        manager
            .process_fetch_request(signers[0].author(), req)
            .await;

        // Should have sent a response
        let msgs = network.sent_messages().await;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].name(), "FetchResponse");
    }

    #[tokio::test]
    async fn test_fetch_request_unknown_cert() {
        let (mut signers, verifier) = create_test_validators(4);
        let (mut manager, network) = create_test_manager(&mut signers, verifier);
        manager.start_view1().await.unwrap();
        network.sent.lock().await.clear();

        // Request for unknown cert
        let req = CertFetchRequest::new(HashValue::random(), 1, 0);
        manager
            .process_fetch_request(signers[0].author(), req)
            .await;

        // Should NOT have sent a response (don't reply if we don't have it)
        assert!(network.sent_messages().await.is_empty());
    }
}
