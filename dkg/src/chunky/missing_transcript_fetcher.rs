// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::{
        common::deserialize_chunky_transcript_and_verify,
        transcript_cache::TranscriptCache,
        types::{ChunkyTranscriptWithHash, MissingTranscriptRequest},
    },
    network::NetworkSender,
    DKGMessage,
};
use anyhow::{anyhow, ensure, Result};
use aptos_crypto::HashValue;
use aptos_infallible::RwLock;
use aptos_logger::warn;
use aptos_types::{
    dkg::chunky_dkg::{ChunkyDKGSession, DealerPublicKey},
    epoch_state::EpochState,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, sync::Arc, time::Duration};

/// Maximum number of retries per dealer before giving up.
const MAX_RETRIES: usize = 10;

const RETRY_DELAY: Duration = Duration::from_millis(500);
// Leave room in the per-peer DKG RPC queue for certification and other requests.
const MAX_CONCURRENT_FETCHES: usize = 4;

/// Fetches transcripts from a specific peer via RPC. Handles both missing and equivocated
/// transcripts (where the local copy differs from the requester's).
pub struct TranscriptFetcher {
    sender: AccountAddress,
    epoch: u64,
    missing_dealers: Vec<(AccountAddress, HashValue)>,
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
        missing_dealers: Vec<(AccountAddress, HashValue)>,
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
    pub(crate) async fn run(
        &self,
        network_sender: Arc<NetworkSender>,
        cache: Arc<RwLock<TranscriptCache>>,
    ) -> Result<HashMap<AccountAddress, ChunkyTranscriptWithHash>> {
        let mut missing: HashMap<AccountAddress, HashValue> =
            self.missing_dealers.iter().copied().collect();
        let mut results: HashMap<AccountAddress, ChunkyTranscriptWithHash> = HashMap::new();

        let mut pending_requests: FuturesUnordered<RpcFuture> = FuturesUnordered::new();

        let mut remaining = self.missing_dealers.iter();
        // Bound requests to this peer instead of flooding its RPC queue.
        for &(dealer_addr, _) in remaining.by_ref().take(MAX_CONCURRENT_FETCHES) {
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
            .map(|info| {
                info.public_key()
                    .expect("on-chain dealer consensus keys must be valid")
                    .clone()
            })
            .collect();

        while !missing.is_empty() && !pending_requests.is_empty() {
            let Some((dealer_addr, attempt, result)) = pending_requests.next().await else {
                break;
            };

            let expected_hash = missing[&dealer_addr];
            match self.process_response(dealer_addr, expected_hash, result, &signing_pubkeys) {
                Ok((transcript, serialized_bytes)) => {
                    // Publish each verified result immediately. Partial progress must
                    // survive another fetch failing or the enclosing handler timing out.
                    cache
                        .write()
                        .insert(dealer_addr, transcript.clone(), serialized_bytes);
                    missing.remove(&dealer_addr);
                    results.insert(dealer_addr, transcript);
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
                        continue;
                    }
                },
            }
            if let Some(&(dealer_addr, _)) = remaining.next() {
                pending_requests.push(self.create_request_future(
                    dealer_addr,
                    0,
                    network_sender.clone(),
                    None,
                ));
            }
        }

        if !missing.is_empty() {
            return Err(anyhow!(
                "Failed to fetch all transcripts. Still missing: {:?}",
                missing.keys()
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
        expected_hash: HashValue,
        result: Result<DKGMessage>,
        signing_pubkeys: &[DealerPublicKey],
    ) -> Result<(ChunkyTranscriptWithHash, usize)> {
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

        let serialized_bytes = transcript_response.transcript_bytes.len();
        ensure!(
            serialized_bytes <= self.dkg_config.expected_max_transcript_size(),
            "fetched transcript exceeds session size limit"
        );
        ensure!(
            HashValue::sha3_256_of(&transcript_response.transcript_bytes) == expected_hash,
            "fetched transcript does not match requested hash"
        );
        let transcript = deserialize_chunky_transcript_and_verify(
            dealer_addr,
            &transcript_response.transcript_bytes,
            &self.dkg_config,
            signing_pubkeys,
            &self.epoch_state,
        )?;
        Ok((transcript, serialized_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunky::{
        test_utils::{loopback_network, ChunkyTestSetup},
        types::MissingTranscriptResponse,
    };
    use aptos_network::protocols::network::Event;

    #[tokio::test]
    async fn partial_progress_survives_fetch_cancellation() {
        let setup = ChunkyTestSetup::new_uniform(4);
        let (wire, _) = setup.deal_transcript(0);
        let hash = HashValue::sha3_256_of(&wire.transcript_bytes);
        let source = setup.addrs[3];
        let (network, mut incoming) = loopback_network(source);
        let fetcher = TranscriptFetcher::new(
            source,
            setup.epoch_state.epoch,
            vec![(setup.addrs[0], hash), (setup.addrs[1], HashValue::zero())],
            Duration::from_secs(10),
            setup.dkg_config.clone(),
            setup.epoch_state.clone(),
        );
        let cache = Arc::new(RwLock::new(TranscriptCache::default()));
        let task_cache = cache.clone();
        let fetch_task = tokio::spawn(async move { fetcher.run(network, task_cache).await });
        let provider = tokio::spawn(async move {
            let mut pending_responses = Vec::new();
            for _ in 0..2 {
                let Event::RpcRequest(_, DKGMessage::MissingTranscriptRequest(req), protocol, tx) =
                    incoming.next().await.unwrap()
                else {
                    panic!("expected fetch request")
                };
                if req.missing_dealer == wire.metadata.author {
                    let response = DKGMessage::MissingTranscriptResponse(
                        MissingTranscriptResponse::new(wire.clone()),
                    );
                    tx.send(Ok(protocol.to_bytes(&response).unwrap().into()))
                        .unwrap();
                } else {
                    pending_responses.push(tx);
                }
            }
            std::future::pending::<()>().await;
            drop(pending_responses);
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            while cache.read().get(setup.addrs[0], hash).is_none() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(!fetch_task.is_finished());
        fetch_task.abort();
        assert!(fetch_task.await.err().unwrap().is_cancelled());
        provider.abort();
        assert!(cache.read().get(setup.addrs[0], hash).is_some());
        assert!(cache
            .read()
            .get(setup.addrs[1], HashValue::zero())
            .is_none());
    }

    #[test]
    fn rejects_wrong_variant_and_invalid_transcript() {
        let setup = ChunkyTestSetup::new_uniform(4);
        let (mut wire, _) = setup.deal_transcript(0);
        let hash = HashValue::sha3_256_of(&wire.transcript_bytes);
        let fetcher = TranscriptFetcher::new(
            setup.addrs[3],
            setup.epoch_state.epoch,
            vec![(setup.addrs[0], hash)],
            Duration::from_secs(10),
            setup.dkg_config.clone(),
            setup.epoch_state.clone(),
        );
        let result = fetcher.process_response(
            setup.addrs[0],
            HashValue::zero(),
            Ok(DKGMessage::MissingTranscriptResponse(
                MissingTranscriptResponse::new(wire.clone()),
            )),
            &setup.public_keys,
        );
        assert!(result.err().unwrap().to_string().contains("requested hash"));
        wire.transcript_bytes[0] ^= 0xFF;
        let invalid_hash = HashValue::sha3_256_of(&wire.transcript_bytes);
        assert!(fetcher
            .process_response(
                setup.addrs[0],
                invalid_hash,
                Ok(DKGMessage::MissingTranscriptResponse(
                    MissingTranscriptResponse::new(wire)
                )),
                &setup.public_keys,
            )
            .is_err());
    }
}
