// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator observation and quorum certification for governed trust targets.

use crate::{
    network::IncomingRpcRequest,
    trust_observer::TrustObserver,
    types::{JWKConsensusMsg, TrustObservationRequest, TrustObservationResponse},
};
use anyhow::{anyhow, ensure, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::common::Author;
use aptos_crypto::{
    bls12381::{PrivateKey, Signature},
    SigningKey,
};
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, info};
use aptos_reliable_broadcast::{BroadcastStatus, ReliableBroadcast};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::PartialSignatures,
    epoch_state::EpochState,
    trust_observation::{
        Config, ObservedTrustData, ObservedTrustDataUpdated, ObservedValue, QuorumCertifiedUpdate,
        TargetSpec,
    },
    validator_txn::{Topic, ValidatorTransaction},
    validator_verifier::VerifyError,
};
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use futures_channel::oneshot;
use futures_util::{
    future::{join_all, AbortHandle, Abortable},
    FutureExt, StreamExt,
};
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};
use tokio_retry::strategy::ExponentialBackoff;

type TrustBroadcast = ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>;

struct TargetState {
    onchain: Option<ObservedValue>,
    latest_observation: Option<Vec<u8>>,
    proposal: Option<ObservedValue>,
    signature: Option<Signature>,
    abort_handle: Option<AbortHandle>,
    vtxn_guard: Option<TxnGuard>,
}

impl TargetState {
    fn new(onchain: Option<ObservedValue>) -> Self {
        Self {
            onchain,
            latest_observation: None,
            proposal: None,
            signature: None,
            abort_handle: None,
            vtxn_guard: None,
        }
    }

    fn clear_proposal(&mut self) {
        if let Some(handle) = self.abort_handle.take() {
            handle.abort();
        }
        self.vtxn_guard = None;
        self.proposal = None;
        self.signature = None;
    }
}

pub struct TrustObservationManager {
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,
    consensus_key: Arc<PrivateKey>,
    rb: Arc<TrustBroadcast>,
    vtxn_pool: VTxnPoolState,
    targets: HashMap<Vec<u8>, TargetSpec>,
    states: HashMap<Vec<u8>, TargetState>,
    extra_root_certificates_pem: Vec<String>,
    observers: Vec<TrustObserver>,
    qc_tx: aptos_channel::Sender<Vec<u8>, QuorumCertifiedUpdate>,
    qc_rx: aptos_channel::Receiver<Vec<u8>, QuorumCertifiedUpdate>,
}

impl TrustObservationManager {
    pub fn new(
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        consensus_key: Arc<PrivateKey>,
        rb: TrustBroadcast,
        vtxn_pool: VTxnPoolState,
        config: Config,
        observed: ObservedTrustData,
        extra_root_certificates_pem: Vec<String>,
    ) -> Self {
        let (qc_tx, qc_rx) = aptos_channel::new(QueueStyle::KLAST, 100, None);
        let states = config
            .targets
            .iter()
            .map(|target| {
                let onchain = observed
                    .get(&target.id)
                    .filter(|entry| entry.config_version == target.config_version)
                    .cloned();
                (target.id.clone(), TargetState::new(onchain))
            })
            .collect();
        Self {
            my_addr,
            epoch_state,
            consensus_key,
            rb: Arc::new(rb),
            vtxn_pool,
            targets: config
                .targets
                .into_iter()
                .map(|target| (target.id.clone(), target))
                .collect(),
            states,
            extra_root_certificates_pem,
            observers: Vec::new(),
            qc_tx,
            qc_rx,
        }
    }

    fn process_observation(&mut self, target_id: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let target = self
            .targets
            .get(&target_id)
            .ok_or_else(|| anyhow!("observation for unconfigured target"))?;
        ensure!(
            value.len() <= target.max_bytes as usize,
            "observation exceeds target limit"
        );
        let state = self
            .states
            .get_mut(&target_id)
            .ok_or_else(|| anyhow!("missing target state"))?;
        state.latest_observation = Some(value.clone());
        if state.onchain.as_ref().is_some_and(|current| {
            current.config_version == target.config_version && current.value == value
        }) {
            state.clear_proposal();
            return Ok(());
        }
        let next_version = match state.onchain.as_ref() {
            Some(current) => current
                .version
                .checked_add(1)
                .ok_or_else(|| anyhow!("trust observation version exhausted"))?,
            None => 1,
        };
        let proposal = ObservedValue {
            target_id: target_id.clone(),
            config_version: target.config_version,
            epoch: self.epoch_state.epoch,
            version: next_version,
            value,
        };
        if state.proposal.as_ref() == Some(&proposal) {
            return Ok(());
        }
        state.clear_proposal();
        let signature = self.consensus_key.sign(&proposal)?;
        let request = TrustObservationRequest {
            epoch: self.epoch_state.epoch,
            target_id: target_id.clone(),
            config_version: proposal.config_version,
            version: proposal.version,
        };
        let rb = self.rb.clone();
        let aggregate = Arc::new(TrustObservationAggregationState::new(
            self.epoch_state.clone(),
            proposal.clone(),
        ));
        let qc_tx = self.qc_tx.clone();
        let task = async move {
            match rb.broadcast(request, aggregate).await {
                Ok(qc) => {
                    let _ = qc_tx.push(target_id.clone(), qc);
                },
                Err(error) => {
                    error!(target_id = ?target_id, "Trust observation certification failed: {error:#}")
                },
            }
        };
        let (abort_handle, registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, registration));
        state.proposal = Some(proposal);
        state.signature = Some(signature);
        state.abort_handle = Some(abort_handle);
        Ok(())
    }

    fn process_peer_request(&self, rpc: IncomingRpcRequest) -> Result<()> {
        let IncomingRpcRequest {
            msg,
            mut response_sender,
            ..
        } = rpc;
        let JWKConsensusMsg::TrustObservationRequest(request) = msg else {
            response_sender.send(Err(anyhow!("unexpected trust observation RPC")));
            return Ok(());
        };
        let response = (|| {
            ensure!(
                request.epoch == self.epoch_state.epoch,
                "stale trust observation epoch"
            );
            let target = self
                .targets
                .get(&request.target_id)
                .ok_or_else(|| anyhow!("unconfigured trust target"))?;
            ensure!(
                target.config_version == request.config_version,
                "target configuration changed"
            );
            let state = self
                .states
                .get(&request.target_id)
                .ok_or_else(|| anyhow!("target state unavailable"))?;
            let observed = state
                .proposal
                .clone()
                .ok_or_else(|| anyhow!("local trust observation unavailable"))?;
            ensure!(
                observed.version == request.version,
                "proposed version mismatch"
            );
            ensure!(observed.epoch == request.epoch, "proposed epoch mismatch");
            let signature = state
                .signature
                .clone()
                .ok_or_else(|| anyhow!("local trust signature unavailable"))?;
            Ok(JWKConsensusMsg::TrustObservationResponse(
                TrustObservationResponse {
                    epoch: self.epoch_state.epoch,
                    author: self.my_addr,
                    observed,
                    signature,
                },
            ))
        })();
        response_sender.send(response);
        Ok(())
    }

    fn process_qc(&mut self, target_id: Vec<u8>, qc: QuorumCertifiedUpdate) -> Result<()> {
        let state = self
            .states
            .get_mut(&target_id)
            .ok_or_else(|| anyhow!("certified unknown trust target"))?;
        ensure!(
            state.proposal.as_ref() == Some(&qc.update),
            "certified value is no longer current"
        );
        ensure!(state.vtxn_guard.is_none(), "trust update already queued");
        let txn = ValidatorTransaction::ObservedTrustUpdate(qc);
        state.vtxn_guard = Some(self.vtxn_pool.put(
            Topic::TRUST_OBSERVATION(target_id.clone()),
            Arc::new(txn),
            None,
        ));
        info!(target_id = ?target_id, "Certified trust observation queued as validator transaction");
        Ok(())
    }

    fn process_onchain_event(&mut self, event: ObservedTrustDataUpdated) -> Result<()> {
        let observed = event.observed;
        let id = observed.target_id.clone();
        let Some(target) = self.targets.get(&id) else {
            return Ok(());
        };
        let state = match self.states.get_mut(&id) {
            Some(state) => state,
            None => return Ok(()),
        };
        if !should_apply_event(
            target,
            self.epoch_state.epoch,
            state.onchain.as_ref(),
            &observed,
        ) {
            debug!(target_id = ?id, "Ignoring stale trust observation event");
            return Ok(());
        }
        state.onchain = Some(observed);
        state.clear_proposal();
        if let Some(value) = state.latest_observation.clone() {
            self.process_observation(id, value)?;
        }
        Ok(())
    }

    pub async fn run(
        mut self,
        mut event_rx: aptos_channel::Receiver<(), ObservedTrustDataUpdated>,
        mut rpc_rx: aptos_channel::Receiver<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        let (observation_tx, mut observation_rx) = aptos_channel::new(QueueStyle::KLAST, 100, None);
        self.observers = self
            .targets
            .values()
            .cloned()
            .map(|target| {
                TrustObserver::spawn(
                    self.epoch_state.epoch,
                    target,
                    Duration::from_secs(10),
                    observation_tx.clone(),
                    self.extra_root_certificates_pem.clone(),
                )
            })
            .collect();
        let mut close_rx = close_rx.into_stream();
        loop {
            let result = tokio::select! {
                (id, value) = observation_rx.select_next_some() => self.process_observation(id, value),
                (_, rpc) = rpc_rx.select_next_some() => self.process_peer_request(rpc),
                qc = self.qc_rx.select_next_some() => self.process_qc(qc.update.target_id.clone(), qc),
                event = event_rx.select_next_some() => self.process_onchain_event(event),
                ack = close_rx.select_next_some() => {
                    for state in self.states.values_mut() { state.clear_proposal(); }
                    join_all(std::mem::take(&mut self.observers).into_iter().map(TrustObserver::shutdown)).await;
                    if let Ok(ack) = ack { let _ = ack.send(()); }
                    break;
                },
            };
            if let Err(error) = result {
                error!("Trust observation manager error: {error:#}");
            }
        }
    }
}

fn should_apply_event(
    target: &TargetSpec,
    epoch: u64,
    current: Option<&ObservedValue>,
    event: &ObservedValue,
) -> bool {
    event.target_id == target.id
        && event.config_version == target.config_version
        && event.epoch == epoch
        && current.is_none_or(|old| {
            old.config_version == event.config_version && event.version > old.version
        })
}

struct TrustObservationAggregationState {
    epoch_state: Arc<EpochState>,
    local_value: ObservedValue,
    partial_signatures: Mutex<PartialSignatures>,
}

impl TrustObservationAggregationState {
    fn new(epoch_state: Arc<EpochState>, local_value: ObservedValue) -> Self {
        Self {
            epoch_state,
            local_value,
            partial_signatures: Mutex::new(PartialSignatures::empty()),
        }
    }
}

impl BroadcastStatus<JWKConsensusMsg> for Arc<TrustObservationAggregationState> {
    type Aggregated = QuorumCertifiedUpdate;
    type Message = TrustObservationRequest;
    type Response = TrustObservationResponse;

    fn add(&self, sender: Author, response: Self::Response) -> Result<Option<Self::Aggregated>> {
        ensure!(
            response.epoch == self.epoch_state.epoch,
            "peer trust observation from wrong epoch"
        );
        ensure!(
            response.author == sender,
            "peer trust observation author mismatch"
        );
        ensure!(
            response.observed == self.local_value,
            "peer trust observation value mismatch"
        );
        let mut signatures = self.partial_signatures.lock();
        if signatures.contains_voter(&sender) {
            return Ok(None);
        }
        self.epoch_state
            .verifier
            .verify(sender, &self.local_value, &response.signature)?;
        signatures.add_signature(sender, response.signature);
        let voters: BTreeSet<_> = signatures.signatures().keys().copied().collect();
        match self
            .epoch_state
            .verifier
            .check_voting_power(voters.iter(), true)
        {
            Ok(_) => {},
            Err(VerifyError::TooLittleVotingPower { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        let multi_sig = self
            .epoch_state
            .verifier
            .aggregate_signatures(signatures.signatures_iter())?;
        debug!(target_id = ?self.local_value.target_id, "Trust observation reached quorum");
        Ok(Some(QuorumCertifiedUpdate {
            update: self.local_value.clone(),
            multi_sig,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{bls12381, Uniform};
    use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier};

    #[test]
    fn quorum_requires_matching_target_config_and_value() {
        let keys: Vec<_> = (0..4)
            .map(|_| bls12381::PrivateKey::generate_for_testing())
            .collect();
        let addrs: Vec<_> = (0..4).map(|_| AccountAddress::random()).collect();
        let infos = keys
            .iter()
            .enumerate()
            .map(|(index, key)| {
                ValidatorConsensusInfo::new(addrs[index], bls12381::PublicKey::from(key), 1)
            })
            .collect();
        let epoch = Arc::new(EpochState::new(7, ValidatorVerifier::new(infos)));
        let value = ObservedValue {
            target_id: b"intel-root".to_vec(),
            config_version: 2,
            epoch: 7,
            version: 3,
            value: vec![0x30, 0],
        };
        let agg = Arc::new(TrustObservationAggregationState::new(epoch, value.clone()));
        let mut wrong = value.clone();
        wrong.config_version += 1;
        assert!(agg
            .add(addrs[0], TrustObservationResponse {
                epoch: 7,
                author: addrs[0],
                observed: wrong.clone(),
                signature: keys[0].sign(&wrong).unwrap()
            })
            .is_err());
        for i in 0..2 {
            assert!(agg
                .add(addrs[i], TrustObservationResponse {
                    epoch: 7,
                    author: addrs[i],
                    observed: value.clone(),
                    signature: keys[i].sign(&value).unwrap()
                })
                .unwrap()
                .is_none());
        }
        let qc = agg
            .add(addrs[2], TrustObservationResponse {
                epoch: 7,
                author: addrs[2],
                observed: value.clone(),
                signature: keys[2].sign(&value).unwrap(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(qc.update, value);
        assert!(agg
            .epoch_state
            .verifier
            .verify_multi_signatures(&qc.update, &qc.multi_sig)
            .is_ok());
    }

    #[test]
    fn stale_events_cannot_poison_new_epoch_or_config() {
        let target = TargetSpec {
            id: b"root".to_vec(),
            url: "https://example.com/root.der".into(),
            format: 1,
            max_bytes: 1024,
            config_version: 3,
        };
        let event = ObservedValue {
            target_id: target.id.clone(),
            config_version: 3,
            epoch: 20,
            version: 2,
            value: vec![0x30, 0],
        };
        assert!(should_apply_event(&target, 20, None, &event));
        assert!(!should_apply_event(&target, 21, None, &event));
        let mut old_config = event.clone();
        old_config.config_version = 2;
        assert!(!should_apply_event(&target, 20, None, &old_config));
        assert!(!should_apply_event(&target, 20, Some(&event), &event));
    }
}
