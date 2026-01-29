// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{chunky::types::MissingTranscriptRequest, network::NetworkSender, DKGMessage};
use anyhow::{anyhow, Result};
use aptos_dkg::pvss::traits::transcript::HasAggregatableSubtranscript;
use aptos_logger::warn;
use aptos_types::dkg::chunky_dkg::{ChunkyDKGConfig, ChunkyTranscript};
use futures_util::{stream::FuturesUnordered, StreamExt};
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

#[allow(dead_code)]
/// Fetcher for missing transcripts using RPC requests to peers.
pub struct MissingTranscriptFetcher {
    sender: AccountAddress,
    epoch: u64,
    missing_dealers: Vec<AccountAddress>,
    rpc_timeout: Duration,
    dkg_config: ChunkyDKGConfig,
}

#[allow(dead_code)]
impl MissingTranscriptFetcher {
    pub fn new(
        sender: AccountAddress,
        epoch: u64,
        missing_dealers: Vec<AccountAddress>,
        rpc_timeout: Duration,
        dkg_config: ChunkyDKGConfig,
    ) -> Self {
        Self {
            sender,
            epoch,
            missing_dealers,
            rpc_timeout,
            dkg_config,
        }
    }

    /// Run the fetcher to retrieve missing transcripts from peers.
    /// Uses FuturesUnordered in a select! loop to enqueue RPC requests and wait for responses.
    /// Sends one request per missing dealer to avoid large responses.
    /// Retries failed RPC requests with a delay indefinitely until successful.
    /// Returns a map of dealer addresses to their transcripts once all missing transcripts are received.
    pub async fn run(
        &self,
        network_sender: Arc<NetworkSender>,
    ) -> Result<HashMap<AccountAddress, ChunkyTranscript>> {
        // Track which dealers we still need transcripts for
        let mut missing_set: HashSet<AccountAddress> =
            self.missing_dealers.iter().cloned().collect();
        let mut results: HashMap<AccountAddress, ChunkyTranscript> = HashMap::new();
        const RETRY_DELAY: Duration = Duration::from_millis(500);

        // Use FuturesUnordered to queue RPC request futures
        let mut pending_requests: FuturesUnordered<
            std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = (AccountAddress, Result<DKGMessage, anyhow::Error>),
                        > + Send,
                >,
            >,
        > = FuturesUnordered::new();

        // Helper function to create an RPC request future (optionally with delay)
        let create_request_future = |dealer_addr: AccountAddress,
                                     peer: AccountAddress,
                                     epoch: u64,
                                     network_sender: Arc<NetworkSender>,
                                     timeout: Duration,
                                     delay: Option<Duration>| {
            Box::pin(async move {
                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }
                let request = DKGMessage::MissingTranscriptRequest(MissingTranscriptRequest::new(
                    epoch,
                    dealer_addr, // One dealer per request
                ));
                let result = network_sender.send_rpc(peer, request, timeout).await;
                (dealer_addr, result)
            })
        };

        // Enqueue initial RPC requests per missing dealer
        for dealer_addr in &self.missing_dealers {
            let future = create_request_future(
                *dealer_addr,
                self.sender,
                self.epoch,
                network_sender.clone(),
                self.rpc_timeout,
                None, // No delay for initial requests
            );
            pending_requests.push(future);
        }

        // Signing pubkeys for transcript verification (derived from session metadata)
        let signing_pubkeys: Vec<_> = self
            .dkg_config
            .session_metadata
            .dealer_consensus_infos_cloned()
            .into_iter()
            .map(|info| info.public_key)
            .collect();

        // Process responses in a loop until we have all missing transcripts
        loop {
            if missing_set.is_empty() {
                break;
            }
            if pending_requests.is_empty() {
                break;
            }

            tokio::select! {
                Some((dealer_addr, result)) = pending_requests.next() => {
                    match result {
                        Ok(response) => {
                            // Process the response
                            match response {
                                DKGMessage::MissingTranscriptResponse(response) => {
                                    // Process the response - contains a single transcript
                                    let transcript_response = response.transcript;
                                    // TODO(ibalajiarun): Is it enough to check epoch and author without
                                    // an actual signature over them?
                                    if transcript_response.metadata.epoch == self.epoch
                                        && transcript_response.metadata.author == dealer_addr
                                    {
                                        if let Ok(transcript) = bcs::from_bytes::<ChunkyTranscript>(
                                            &transcript_response.transcript_bytes,
                                        ) {
                                            // TODO(ibalajiarun): There is indexing in verify method which can panic.
                                            if transcript
                                                .verify(
                                                    &self.dkg_config.threshold_config,
                                                    &self.dkg_config.public_parameters,
                                                    &signing_pubkeys,
                                                    &self.dkg_config.eks,
                                                    &self.dkg_config.session_metadata,
                                                )
                                                .is_ok()
                                            {
                                                if missing_set.contains(&dealer_addr) {
                                                    results.insert(dealer_addr, transcript);
                                                    missing_set.remove(&dealer_addr);
                                                }
                                            } else {
                                                // Verification failed - retry
                                                warn!(
                                                    "[ChunkyDKG] Transcript verification failed for dealer {}, retrying",
                                                    dealer_addr
                                                );
                                                let future = create_request_future(
                                                    dealer_addr,
                                                    self.sender,
                                                    self.epoch,
                                                    network_sender.clone(),
                                                    self.rpc_timeout,
                                                    Some(RETRY_DELAY),
                                                );
                                                pending_requests.push(future);
                                            }
                                        } else {
                                            // Failed to deserialize - retry
                                            warn!(
                                                "[ChunkyDKG] Failed to deserialize transcript for dealer {}, retrying",
                                                dealer_addr
                                            );
                                            let future = create_request_future(
                                                dealer_addr,
                                                self.sender,
                                                self.epoch,
                                                network_sender.clone(),
                                                self.rpc_timeout,
                                                Some(RETRY_DELAY),
                                            );
                                            pending_requests.push(future);
                                        }
                                    } else {
                                        // Epoch or author mismatch - retry
                                        warn!(
                                            "[ChunkyDKG] Transcript metadata mismatch for dealer {} (expected epoch {}, author {}), got epoch {} author {}, retrying",
                                            dealer_addr,
                                            self.epoch,
                                            dealer_addr,
                                            transcript_response.metadata.epoch,
                                            transcript_response.metadata.author
                                        );
                                        let future = create_request_future(
                                            dealer_addr,
                                            self.sender,
                                            self.epoch,
                                            network_sender.clone(),
                                            self.rpc_timeout,
                                            Some(RETRY_DELAY),
                                        );
                                        pending_requests.push(future);
                                    }
                                },
                                _ => {
                                    // Unexpected message type - retry
                                    warn!(
                                        "[ChunkyDKG] Unexpected message type for dealer {}, retrying",
                                        dealer_addr
                                    );
                                    let future = create_request_future(
                                        dealer_addr,
                                        self.sender,
                                        self.epoch,
                                        network_sender.clone(),
                                        self.rpc_timeout,
                                        Some(RETRY_DELAY),
                                    );
                                    pending_requests.push(future);
                                },
                            }
                        },
                        Err(e) => {
                            // Retry with delay
                            warn!(
                                "[ChunkyDKG] Error fetching transcript for dealer {} from peer {}, retrying: {}",
                                dealer_addr, self.sender, e
                            );
                            // Requeue with delay
                            let future = create_request_future(
                                dealer_addr,
                                self.sender,
                                self.epoch,
                                network_sender.clone(),
                                self.rpc_timeout,
                                Some(RETRY_DELAY), // Delay before retry
                            );
                            pending_requests.push(future);
                        },
                    }
                },
            }
        }

        // Check if we got all missing transcripts
        if !missing_set.is_empty() {
            return Err(anyhow!(
                "Failed to fetch all missing transcripts. Still missing: {:?}",
                missing_set
            ));
        }

        Ok(results)
    }
}
