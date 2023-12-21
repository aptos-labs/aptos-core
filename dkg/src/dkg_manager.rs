// Copyright © Aptos Foundation

use crate::{
    network::IncomingDKGRequest,
    network_interface::DKGMsg,
    tracing::{observe_dkg, DKGStage},
    types::{DKGAggNodeAck, DKGAggNodeAckState, DKGNodeAck, DKGNodeAckState, TDKGMessage},
    DKGMessage, DKGNode,
};
use anyhow::{anyhow, bail, ensure, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::{bls12381, Uniform};
use aptos_dkg::pvss::{traits::Transcript, Player};
use aptos_infallible::duration_since_epoch;
use aptos_logger::{debug, error};
use aptos_network::protocols::network::RpcError;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    dkg::{
        build_dkg_pvss_config, DKGAggNode, DKGPvssConfig, DKGTranscriptWrapper, StartDKGEvent, WTrx,
    },
    epoch_state::EpochState,
    validator_txn::ValidatorTransaction,
    validator_verifier::VerifyError,
};
use aptos_validator_transaction_pool as vtxn_pool;
use bytes::Bytes;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt, StreamExt,
};
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use rand::{rngs::StdRng, thread_rng, SeedableRng};
use std::{collections::HashSet, sync::Arc};
use tokio_retry::strategy::ExponentialBackoff;

pub struct DKGManager {
    my_addr: AccountAddress,
    epoch_state: EpochState,
    aggregation_request_rx: aptos_channel::Receiver<u64, ()>,
    aggregation_request_tx: aptos_channel::Sender<u64, ()>,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    private_key: bls12381::PrivateKey,
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,

    pvss_config: Option<Arc<DKGPvssConfig>>,
    my_node: Option<Arc<DKGNode>>,
    agg_trx: Option<Arc<DKGTranscriptWrapper>>,
    contributors: HashSet<AccountAddress>,
    agg_node: Option<Arc<DKGAggNode>>,
    start_time_us: Option<u64>,
    my_node_broadcast: Option<AbortHandle>,
    agg_node_broadcast: Option<AbortHandle>,
}

impl DKGManager {
    pub fn new(
        my_addr: AccountAddress,
        epoch_state: EpochState,
        private_key: bls12381::PrivateKey,
        reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
        vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    ) -> Self {
        let (aggregation_request_tx, aggregation_request_rx) =
            aptos_channel::new(QueueStyle::KLAST, 1, None);
        Self {
            my_addr,
            epoch_state,
            aggregation_request_rx,
            aggregation_request_tx,
            vtxn_pool_write_cli,
            contributors: HashSet::new(),
            my_node: None,
            agg_node: None,
            private_key,
            start_time_us: None,
            my_node_broadcast: None,
            agg_node_broadcast: None,
            reliable_broadcast,
            pvss_config: None,
            agg_trx: None,
        }
    }

    pub async fn run(
        mut self,
        mut start_dkg_event_rx: aptos_channel::Receiver<u64, StartDKGEvent>,
        mut rpc_msg_rx: aptos_channel::Receiver<u64, (AccountAddress, IncomingDKGRequest)>,
        mut dkg_txn_pulled_rx: vtxn_pool::PullNotificationReceiver,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        //dkg todo: load states from db

        let mut close_rx = close_rx.into_stream();

        loop {
            let handling_err = tokio::select! {
                start_dkg_event = start_dkg_event_rx.select_next_some() => {
                    self.process_start_dkg_event(start_dkg_event).await.err()
                },
                (_sender, msg) = rpc_msg_rx.select_next_some() => {
                    self.process_peer_rpc_msg(msg).await.err()
                },
                _ = self.aggregation_request_rx.select_next_some() => {
                    self.try_form_agg_node().await.err()
                },
                _dkg_txn = dkg_txn_pulled_rx.select_next_some() => {
                    debug!("[DKG] transcript proposed");
                    observe_dkg(self.start_time_us, DKGStage::DKG_AGG_NODE_PROPOSED);
                    None
                },
                close_req = close_rx.select_next_some() => {
                    debug!("[DKG] main: tearing down");
                    observe_dkg(self.start_time_us, DKGStage::DKG_FINISH);
                    self.vtxn_pool_write_cli.put(None);
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).unwrap();
                    }
                    break;
                }
            };

            if let Some(err) = handling_err {
                error!("[DKG] handling error: {err}");
            }

            let has_pvss_config = self.load_pvss_config().await.unwrap().is_some();
            let has_my_node = self.load_my_node().await.unwrap().is_some();
            let has_agg_node = self.load_agg_node().await.unwrap().is_some();
            let broadcasting_my_node = self.my_node_broadcast.is_some();
            let broadcasting_agg_node = self.agg_node_broadcast.is_some();
            let has_aggregating_trx = self.load_aggregating_trx().await.unwrap().is_some();
            let contributors = self.load_contributors().await.unwrap();
            let contributing_power = match self
                .epoch_state
                .verifier
                .check_voting_power(contributors.iter(), false)
            {
                Ok(power) => power,
                Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => voting_power,
                _ => unreachable!(),
            };
            debug!(
                "[DKG] local_state: has_pvss_config={has_pvss_config}, has_my_node={has_my_node}, has_agg_node={has_agg_node}, broadcasting_my_node={broadcasting_my_node}, broadcasting_agg_node={broadcasting_agg_node}, has_aggregating_trx={has_aggregating_trx}, contributing_power={contributing_power}",
            );
        }
    }

    async fn try_form_agg_node(&mut self) -> Result<()> {
        debug!("[DKG] try_form_agg_node: BEGIN");
        let contributors = self.load_contributors().await?;
        self.epoch_state
            .verifier
            .check_voting_power(contributors.iter(), false)?;
        debug!("[DKG] try_form_agg_node: check passed, making changes");
        observe_dkg(self.start_time_us, DKGStage::DKG_AGG_NODE_READY);
        let trx = self.load_aggregating_trx().await?.unwrap();
        let agg_node = DKGAggNode::new(self.epoch(), self.my_addr, trx.as_ref().clone());
        self.save_agg_node(agg_node.clone()).await?;
        let txn = ValidatorTransaction::DKGTranscriptForNextEpoch(agg_node);
        self.vtxn_pool_write_cli.put(Some(Arc::new(txn)));
        self.start_broadcast_agg_node().await?;
        debug!("[DKG] try_form_agg_node: END");
        Ok(())
    }

    async fn process_start_dkg_event(&mut self, event: StartDKGEvent) -> Result<()> {
        debug!(
            "[DKG] process_start_dkg_event: BEGIN: cur_epoch={}, target_epoch={}",
            self.epoch_state.epoch, event.target_epoch
        );
        ensure!(self.epoch_state.epoch + 1 == event.target_epoch);
        ensure!(
            self.should_deal().await?,
            "already have either my own node or an agg node"
        );
        let start_time = duration_since_epoch().as_micros() as u64;
        self.start_time_us = Some(start_time);
        let dkg_pvss_config =
            build_dkg_pvss_config(self.epoch_state.epoch, &event.target_validator_set);
        let my_index = *self
            .epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.my_addr)
            .unwrap();

        let seed = if cfg!(feature = "smoke-test") {
            debug!("[DKG] use smoke test special seed!");
            // In DKG test, the test cases need to get the same input secret, so it can verify the reconstructed dealt secret.
            // See function `verify_dkg_transcript()` in `testsuite/smoke-test/src/dkg/mod.rs`.
            self.private_key.to_bytes()
        } else {
            aptos_dkg::utils::random::random_scalar(&mut thread_rng()).to_bytes_le()
        };

        let mut rng = StdRng::from_seed(seed);

        // The secret generated by the dealer
        let s = <WTrx as Transcript>::InputSecret::generate(&mut rng);
        // The auxiliary information used for PVSS
        let aux = (self.epoch_state.epoch, self.my_addr);

        // compute one transcript for generating the keys for the randomness generation
        let trx = WTrx::deal(
            &dkg_pvss_config.wconfig,
            &dkg_pvss_config.pp,
            &self.private_key,
            &dkg_pvss_config.eks,
            &s,
            &aux,
            &Player { id: my_index },
            &mut rng,
        );

        let dkg_trx_wrapper = DKGTranscriptWrapper { trx };
        let dkg_node = DKGNode::new(
            self.epoch_state.epoch,
            self.my_addr,
            dkg_trx_wrapper.clone(),
        );
        self.save_pvss_config(dkg_pvss_config).await?;
        self.save_my_node(dkg_node).await?;
        self.save_aggregating_trx(dkg_trx_wrapper).await?;
        self.save_contributor(self.my_addr).await?;
        observe_dkg(self.start_time_us, DKGStage::DKG_NODE_READY);
        self.schedule_aggregation()?;
        self.start_broadcast_my_node().await?;
        debug!("[DKG] process_start_dkg_event: OK");
        Ok(())
    }

    fn schedule_aggregation(&mut self) -> Result<()> {
        self.aggregation_request_tx
            .push(0, ())
            .map_err(|e| anyhow!("could not schedule aggregation, caused by: {e}"))
    }

    async fn save_my_node(&mut self, node: DKGNode) -> Result<()> {
        //todo: save to db.
        self.my_node = Some(Arc::new(node));
        Ok(())
    }

    async fn start_broadcast_my_node(&mut self) -> Result<()> {
        debug!("[DKG] start_broadcast_my_node: BEGIN");
        let my_node = self
            .load_my_node()
            .await?
            .ok_or_else(|| anyhow!("i do not have a node to broadcast"))?;

        let agg_node = self.load_agg_node().await?;
        ensure!(
            agg_node.is_none(),
            "already have an agg node, why broadcasting my node???"
        );

        if let Some(handle) = self.my_node_broadcast.take() {
            debug!("[DKG] start_broadcast_my_node: stopping existing broadcast of my node");
            handle.abort();
        }

        let ack_set = Arc::new(DKGNodeAckState::new(self.epoch_state.verifier.len()));
        let rb = self.reliable_broadcast.clone();
        let task = rb.broadcast(my_node.as_ref().clone(), ack_set);
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        self.my_node_broadcast = Some(abort_handle);
        debug!("[DKG] start_broadcast_my_node: END");
        Ok(())
    }

    async fn start_broadcast_agg_node(&mut self) -> Result<()> {
        debug!("[DKG] start_broadcast_agg_node: BEGIN");
        let agg_node = self
            .load_agg_node()
            .await?
            .ok_or_else(|| anyhow!("no agg node to broadcast"))?;

        if let Some(handle) = self.my_node_broadcast.take() {
            debug!("[DKG] start_broadcast_agg_node: stopping existing broadcast of my node");
            handle.abort();
        }

        if let Some(handle) = self.agg_node_broadcast.take() {
            debug!("[DKG] start_broadcast_agg_node: stopping existing broadcast of agg node");
            handle.abort();
        }

        let ack_set = Arc::new(DKGAggNodeAckState::new(self.epoch_state.verifier.len()));
        let rb = self.reliable_broadcast.clone();
        let task = rb.broadcast(agg_node.as_ref().clone(), ack_set);
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        self.agg_node_broadcast = Some(abort_handle);
        debug!("[DKG] start_broadcast_agg_node: END");
        Ok(())
    }

    async fn process_peer_rpc_msg(&mut self, req: IncomingDKGRequest) -> Result<()> {
        debug!("[DKG] process_peer_msg: BEGIN");
        let msg = bcs::from_bytes::<DKGMessage>(req.req.data.as_slice())
            .map_err(|_e| anyhow!("Could not deserialize req.data into DKGMessage"))?;
        ensure!(msg.epoch() == self.epoch());
        let response = match msg {
            DKGMessage::DKGNodeMsg(msg) => {
                DKGMessage::DKGNodeAckMsg(self.process_peer_node(req.sender, msg).await?)
            },
            DKGMessage::DKGAggNodeMsg(msg) => {
                DKGMessage::DKGAggNodeAckMsg(self.process_peer_agg_node(msg).await?)
            },
            _ => {
                bail!("unexpected DKGMessage variant")
            },
        };

        let rpc_response = req
            .protocol
            .to_bytes(&DKGMsg::from(response))
            .map(Bytes::from)
            .map_err(RpcError::ApplicationError);
        let _ = req.response_sender.send(rpc_response); // May not succeed.

        debug!("[DKG] process_peer_msg: END");
        Ok(())
    }

    async fn process_peer_node(
        &mut self,
        peer: AccountAddress,
        node: DKGNode,
    ) -> Result<DKGNodeAck> {
        debug!("[DKG] process_peer_node: BEGIN");
        let pvss_config = self
            .load_pvss_config()
            .await?
            .ok_or_else(|| anyhow!("no pvss config locally"))?;
        ensure!(
            self.load_agg_node().await?.is_none(),
            "ignoring peer node as we have agg node already"
        );
        debug!("peer={}", peer);
        debug!("peer_is_self={}", peer == self.my_addr);
        ensure!(
            !self.has_peer_contributed(&peer).await?,
            "ignoring peer node: he has contributed"
        );
        node.verify(pvss_config.as_ref(), &self.epoch_state.verifier)?;

        debug!("[DKG] process_peer_node: verified everything, making changes.");
        let updated_trx = if let Some(trx) = self.load_aggregating_trx().await? {
            let mut updated_trx = trx.as_ref().clone();
            updated_trx.aggregate_with(pvss_config.as_ref(), node.transcript());
            updated_trx
        } else {
            node.transcript().clone()
        };
        self.save_aggregating_trx(updated_trx).await?;
        self.save_contributor(peer).await?;
        self.schedule_aggregation()?;
        observe_dkg(self.start_time_us, DKGStage::DKG_NODES_RECEIVED);
        debug!("[DKG] process_peer_node: END");
        Ok(DKGNodeAck::new(self.epoch_state.epoch))
    }

    async fn process_peer_agg_node(&mut self, agg_node: DKGAggNode) -> Result<DKGAggNodeAck> {
        debug!("[DKG] process_peer_agg_node: BEGIN");
        let cur_agg_node = self.load_agg_node().await?;
        ensure!(
            cur_agg_node.is_none(),
            "i already have an agg node, ignoring this one from peer"
        );
        let pvss_config = self
            .load_pvss_config()
            .await?
            .ok_or_else(|| anyhow!("no pvss config, could not verify peer agg node"))?;
        agg_node.verify(pvss_config.as_ref(), &self.epoch_state.verifier)?;

        debug!("[DKG] process_peer_agg_node: check passed, making changes");
        self.save_agg_node(agg_node.clone()).await?;
        let txn = ValidatorTransaction::DKGTranscriptForNextEpoch(agg_node);
        self.vtxn_pool_write_cli.put(Some(Arc::new(txn)));
        self.start_broadcast_agg_node().await?;

        debug!("[DKG] process_peer_agg_node: END");
        Ok(DKGAggNodeAck::new(self.epoch()))
    }

    fn epoch(&self) -> u64 {
        self.epoch_state.epoch
    }

    async fn load_aggregating_trx(&self) -> Result<Option<Arc<DKGTranscriptWrapper>>> {
        Ok(self.agg_trx.clone())
    }

    async fn save_aggregating_trx(&mut self, trx: DKGTranscriptWrapper) -> Result<()> {
        self.agg_trx = Some(Arc::new(trx));
        Ok(())
    }

    async fn load_pvss_config(&self) -> Result<Option<Arc<DKGPvssConfig>>> {
        Ok(self.pvss_config.clone())
    }

    async fn save_pvss_config(&mut self, config: DKGPvssConfig) -> Result<()> {
        self.pvss_config = Some(Arc::new(config));
        Ok(())
    }

    async fn has_peer_contributed(&self, peer: &AccountAddress) -> Result<bool> {
        Ok(self.contributors.contains(peer))
    }

    async fn save_contributor(&mut self, contributor: AccountAddress) -> Result<()> {
        self.contributors.insert(contributor);
        Ok(())
    }

    async fn load_contributors(&self) -> Result<HashSet<AccountAddress>> {
        Ok(self.contributors.clone())
    }

    async fn load_my_node(&self) -> Result<Option<Arc<DKGNode>>> {
        Ok(self.my_node.clone())
    }

    async fn load_agg_node(&self) -> Result<Option<Arc<DKGAggNode>>> {
        Ok(self.agg_node.clone())
    }

    async fn save_agg_node(&mut self, agg_node: DKGAggNode) -> Result<()> {
        self.agg_node = Some(Arc::new(agg_node));
        Ok(())
    }

    async fn should_deal(&self) -> Result<bool> {
        Ok(self.load_my_node().await?.is_none() && self.load_agg_node().await?.is_none())
    }
}
