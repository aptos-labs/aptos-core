// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::{
        common::deserialize_chunky_transcript_and_verify,
        types::{ChunkyTranscriptWithHash, MissingTranscriptRequest},
    },
    network::NetworkSender,
    DKGMessage,
};
use anyhow::{anyhow, Result};
use aptos_logger::warn;
use aptos_types::{
    dkg::chunky_dkg::{ChunkyDKGSession, DealerPublicKey},
    epoch_state::EpochState,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

/// Maximum number of retries per dealer before giving up.
const MAX_RETRIES: usize = 10;

const RETRY_DELAY: Duration = Duration::from_millis(500);

/// Fetches transcripts from a specific peer via RPC. Handles both missing and equivocated
/// transcripts (where the local copy differs from the requester's).
pub struct TranscriptFetcher {
    sender: AccountAddress,
    epoch: u64,
    missing_dealers: Vec<AccountAddress>,
    rpc_timeout: Duration,
    dkg_config: Arc<ChunkyDKGSession>,
    epoch_state: Arc<EpochState>,
}

type RpcFuture = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = (AccountAddress, usize, Result<DKGMessage, anyhow::Error>)>
            + Send,
    >,
>;

impl TranscriptFetcher {
    pub fn new(
        sender: AccountAddress,
        epoch: u64,
        missing_dealers: Vec<AccountAddress>,
        rpc_timeout: Duration,
        dkg_config: Arc<ChunkyDKGSession>,
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

    /// Run the fetcher to retrieve transcripts from the peer.
    /// Retries up to MAX_RETRIES per dealer.
    pub async fn run(
        &self,
        network_sender: Arc<NetworkSender>,
    ) -> Result<HashMap<AccountAddress, ChunkyTranscriptWithHash>> {
        let mut missing_set: HashSet<AccountAddress> =
            self.missing_dealers.iter().cloned().collect();
        let mut results: HashMap<AccountAddress, ChunkyTranscriptWithHash> = HashMap::new();

        let mut pending_requests: FuturesUnordered<RpcFuture> = FuturesUnordered::new();

        // Enqueue initial requests (attempt 0)
        for &dealer_addr in &self.missing_dealers {
            pending_requests.push(self.create_request_future(
                dealer_addr,
                0,
                network_sender.clone(),
                None,
            ));
        }

        let signing_pubkeys: Vec<DealerPublicKey> = self
            .dkg_config
            .session_metadata
            .dealer_consensus_infos_cloned()
            .into_iter()
            .map(|info| info.public_key)
            .collect();

        while !missing_set.is_empty() && !pending_requests.is_empty() {
            let Some((dealer_addr, attempt, result)) = pending_requests.next().await else {
                break;
            };

            match self.process_response(dealer_addr, result, &signing_pubkeys) {
                Ok(transcript) => {
                    if missing_set.remove(&dealer_addr) {
                        results.insert(dealer_addr, transcript);
                    }
                },
                Err(e) => {
                    if attempt >= MAX_RETRIES {
                        warn!(
                            "[ChunkyDKG] Giving up on dealer {} after {} retries: {}",
                            dealer_addr, MAX_RETRIES, e
                        );
                    } else {
                        warn!(
                            "[ChunkyDKG] Fetch failed for dealer {} (attempt {}/{}): {}, retrying",
                            dealer_addr,
                            attempt + 1,
                            MAX_RETRIES,
                            e
                        );
                        pending_requests.push(self.create_request_future(
                            dealer_addr,
                            attempt + 1,
                            network_sender.clone(),
                            Some(RETRY_DELAY),
                        ));
                    }
                },
            }
        }

        if !missing_set.is_empty() {
            return Err(anyhow!(
                "Failed to fetch all transcripts. Still missing: {:?}",
                missing_set
            ));
        }

        Ok(results)
    }

    fn create_request_future(
        &self,
        dealer_addr: AccountAddress,
        attempt: usize,
        network_sender: Arc<NetworkSender>,
        delay: Option<Duration>,
    ) -> RpcFuture {
        let peer = self.sender;
        let epoch = self.epoch;
        let timeout = self.rpc_timeout;
        Box::pin(async move {
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            let request = DKGMessage::MissingTranscriptRequest(MissingTranscriptRequest::new(
                epoch,
                dealer_addr,
            ));
            let result = network_sender.send_rpc(peer, request, timeout).await;
            (dealer_addr, attempt, result)
        })
    }

    /// Process a single RPC response, returning the validated transcript or an error to retry.
    fn process_response(
        &self,
        dealer_addr: AccountAddress,
        result: Result<DKGMessage>,
        signing_pubkeys: &[DealerPublicKey],
    ) -> Result<ChunkyTranscriptWithHash> {
        let response = result?;
        let DKGMessage::MissingTranscriptResponse(response) = response else {
            return Err(anyhow!("unexpected message type"));
        };

        let transcript_response = response.transcript;

        // Validate envelope metadata as belt-and-suspenders.
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

        deserialize_chunky_transcript_and_verify(
            dealer_addr,
            &transcript_response.transcript_bytes,
            &self.dkg_config,
            signing_pubkeys,
            &self.epoch_state,
        )
    }
}
