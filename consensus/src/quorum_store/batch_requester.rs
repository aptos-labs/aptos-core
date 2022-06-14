use crate::network::NetworkSender;
use crate::network_interface::ConsensusMsg;
use crate::quorum_store::types::Batch;
use crate::quorum_store::utils::DigestTimeouts;
use aptos_crypto::HashValue;
use aptos_types::{validator_signer::ValidatorSigner, PeerId};
use consensus_types::common::Payload;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::oneshot;

struct BatchRequesterState {
    signers: Vec<PeerId>,
    next_index: usize,
    ret_tx: oneshot::Sender<Payload>,
}

impl BatchRequesterState {
    fn new(signers: Vec<PeerId>, ret_tx: oneshot::Sender<Payload>) -> Self {
        Self {
            signers,
            next_index: 0,
            ret_tx,
        }
    }

    fn next_request_peers(&mut self, num_peers: usize) -> Vec<PeerId> {
        let ret = self
            .signers
            .iter()
            .cycle()
            .skip(self.next_index)
            .take(num_peers)
            .cloned()
            .collect();
        self.next_index = (self.next_index + num_peers) % self.signers.len();
        ret
    }

    fn serve_request(self, payload: Payload) {
        self.ret_tx
            .send(payload)
            .expect("Receiver of requested batch not available");
    }
}

pub(crate) struct BatchRequester {
    epoch: u64,
    my_peer_id: PeerId,
    request_num_peers: usize,
    request_timeout_ms: usize,
    digest_to_state: HashMap<HashValue, BatchRequesterState>,
    timeouts: DigestTimeouts,
    network_sender: NetworkSender,
}

impl BatchRequester {
    pub(crate) fn new(
        epoch: u64,
        my_peer_id: PeerId,
        request_num_peers: usize,
        request_timeout_ms: usize,
        network_sender: NetworkSender,
    ) -> Self {
        Self {
            epoch,
            my_peer_id,
            request_num_peers,
            request_timeout_ms,
            digest_to_state: HashMap::new(),
            timeouts: DigestTimeouts::new(),
            network_sender,
        }
    }

    async fn send_requests(
        &self,
        digest: HashValue,
        request_peers: Vec<PeerId>,
        self_signer: Arc<ValidatorSigner>,
    ) {
        debug_assert!(
            !request_peers.contains(&self.my_peer_id),
            "Should never request from self over network"
        );
        let batch = Batch::new(self.epoch, self.my_peer_id, digest, None, self_signer);
        let msg = ConsensusMsg::BatchMsg(Box::new(batch));
        self.network_sender.send(msg, request_peers).await;
    }

    pub(crate) async fn add_request(
        &mut self,
        digest: HashValue,
        signers: Vec<PeerId>,
        ret_tx: oneshot::Sender<Payload>,
        self_signer: Arc<ValidatorSigner>,
    ) {
        let mut request_state = BatchRequesterState::new(signers, ret_tx);

        let request_peers = request_state.next_request_peers(self.request_num_peers);
        self.send_requests(digest, request_peers, self_signer).await;
        self.digest_to_state.insert(digest, request_state);
        self.timeouts.add_digest(digest, self.request_timeout_ms);
    }

    pub(crate) async fn handle_timeouts(&mut self, self_signer: Arc<ValidatorSigner>) {
        for digest in self.timeouts.expire() {
            if let Some(state) = self.digest_to_state.get_mut(&digest) {
                let request_peers = state.next_request_peers(self.request_num_peers);
                self.send_requests(digest, request_peers, self_signer.clone())
                    .await;
                self.timeouts.add_digest(digest, self.request_timeout_ms);
            }
        }
    }

    // TODO, Rati, how is sending here is not a side effect? Who is it different from the handle messages we had in Quorum_store?
    pub(crate) fn serve_request(&mut self, digest: HashValue, payload: Payload) {
        if self.digest_to_state.contains_key(&digest) {
            let state = self.digest_to_state.remove(&digest).unwrap();
            state.serve_request(payload);
        }
    }
}
