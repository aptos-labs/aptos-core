// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::{types::MissingTranscriptRequest, validation::validate_chunky_transcript},
    network::NetworkSender,
    DKGMessage,
};
use anyhow::{anyhow, Result};
use aptos_logger::warn;
use aptos_types::{
    dkg::chunky_dkg::{ChunkyDKGConfig, ChunkyTranscript, DealerPublicKey},
    epoch_state::EpochState,
};
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
    epoch_state: Arc<EpochState>,
}

#[allow(dead_code)]
impl MissingTranscriptFetcher {
    pub fn new(
        sender: AccountAddress,
        epoch: u64,
        missing_dealers: Vec<AccountAddress>,
        rpc_timeout: Duration,
        dkg_config: ChunkyDKGConfig,
        epoch_state: Arc<EpochState>,
    ) -> Self {
        Self {
            sender,
            epoch,
            missing_dealers,
            rpc_timeout,
            dkg_config,
            epoch_state,
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
        let signing_pubkeys: Vec<DealerPublicKey> = self
            .dkg_config
            .session_metadata
            .dealer_consensus_infos_cloned()
            .into_iter()
            .map(|info| info.public_key)
            .collect();

        // Process responses in a loop until we have all missing transcripts
        loop {
            if missing_set.is_empty() || pending_requests.is_empty() {
                break;
            }

            tokio::select! {
                Some((dealer_addr, result)) = pending_requests.next() => {
                    match self.process_response(dealer_addr, result, &signing_pubkeys) {
                        Ok(transcript) => {
                            if missing_set.remove(&dealer_addr) {
                                results.insert(dealer_addr, transcript);
                            }
                        },
                        Err(e) => {
                            warn!(
                                "[ChunkyDKG] Failed to process transcript for dealer {}: {}, retrying",
                                dealer_addr, e
                            );
                            pending_requests.push(create_request_future(
                                dealer_addr,
                                self.sender,
                                self.epoch,
                                network_sender.clone(),
                                self.rpc_timeout,
                                Some(RETRY_DELAY),
                            ));
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

    /// Process a single RPC response, returning the validated transcript or an error to retry.
    fn process_response(
        &self,
        dealer_addr: AccountAddress,
        result: Result<DKGMessage>,
        signing_pubkeys: &[DealerPublicKey],
    ) -> Result<ChunkyTranscript> {
        let response = result?;
        let DKGMessage::MissingTranscriptResponse(response) = response else {
            return Err(anyhow!("unexpected message type"));
        };

        let transcript_response = response.transcript;

        // Validate envelope metadata (epoch and author) as belt-and-suspenders.
        // The transcript is cryptographically verified via validate_chunky_transcript which
        // checks the dealer's key pair; the dealer-ID check ensures it belongs to the
        // expected dealer.
        if transcript_response.metadata.epoch != self.epoch
            || transcript_response.metadata.author != dealer_addr
        {
            return Err(anyhow!(
                "metadata mismatch: expected epoch {}, author {}, got epoch {} author {}",
                self.epoch,
                dealer_addr,
                transcript_response.metadata.epoch,
                transcript_response.metadata.author,
            ));
        }

        let mut rng = rand::thread_rng();
        validate_chunky_transcript(
            dealer_addr,
            &transcript_response.transcript_bytes,
            &self.dkg_config,
            signing_pubkeys,
            &self.epoch_state,
            &mut rng,
        )
    }
}
