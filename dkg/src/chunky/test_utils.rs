// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::types::DKGMessage;
use aptos_crypto::{
    bls12381::{PrivateKey, PublicKey},
    Uniform,
};
use aptos_dkg::pvss::{traits::transcript::HasAggregatableSubtranscript, Player};
use aptos_reliable_broadcast::RBNetworkSender;
use aptos_types::{
    dkg::chunky_dkg::{
        AggregatedSubtranscript, ChunkyDKG, ChunkyDKGConfig, ChunkyDKGSessionMetadata,
        ChunkyDKGTranscript, ChunkyInputSecret, ChunkyTranscript, DealerPublicKey,
    },
    epoch_state::EpochState,
    on_chain_config::OnChainChunkyDKGConfig,
    validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct},
};
use async_trait::async_trait;
use bytes::Bytes;
use move_core_types::account_address::AccountAddress;
use rand::{prelude::StdRng, thread_rng, SeedableRng};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[allow(dead_code)]
pub struct ChunkyTestSetup {
    pub private_keys: Vec<Arc<PrivateKey>>,
    pub public_keys: Vec<PublicKey>,
    pub addrs: Vec<AccountAddress>,
    pub voting_powers: Vec<u64>,
    pub epoch_state: Arc<EpochState>,
    pub session_metadata: ChunkyDKGSessionMetadata,
    pub dkg_config: ChunkyDKGConfig,
}

impl ChunkyTestSetup {
    pub fn new(n: usize, voting_powers: Vec<u64>) -> Self {
        assert_eq!(n, voting_powers.len());

        let mut rng = thread_rng();
        let private_keys: Vec<Arc<PrivateKey>> = (0..n)
            .map(|_| Arc::new(aptos_crypto::Uniform::generate(&mut rng)))
            .collect();
        let public_keys: Vec<PublicKey> = private_keys
            .iter()
            .map(|sk| PublicKey::from(sk.as_ref()))
            .collect();
        let addrs: Vec<AccountAddress> = (0..n).map(|_| AccountAddress::random()).collect();

        let validator_consensus_infos: Vec<ValidatorConsensusInfo> = (0..n)
            .map(|i| {
                ValidatorConsensusInfo::new(addrs[i], public_keys[i].clone(), voting_powers[i])
            })
            .collect();
        let validator_consensus_info_move_structs: Vec<ValidatorConsensusInfoMoveStruct> =
            validator_consensus_infos
                .iter()
                .cloned()
                .map(ValidatorConsensusInfoMoveStruct::from)
                .collect();

        let verifier =
            aptos_types::validator_verifier::ValidatorVerifier::new(validator_consensus_infos);
        let epoch_state = Arc::new(EpochState::new(999, verifier));

        let session_metadata = ChunkyDKGSessionMetadata {
            dealer_epoch: 999,
            chunky_dkg_config: OnChainChunkyDKGConfig::default_enabled().into(),
            dealer_validator_set: validator_consensus_info_move_structs.clone(),
            target_validator_set: validator_consensus_info_move_structs,
        };
        let dkg_config = ChunkyDKG::generate_config(&session_metadata);

        Self {
            private_keys,
            public_keys,
            addrs,
            voting_powers,
            epoch_state,
            session_metadata,
            dkg_config,
        }
    }

    pub fn new_uniform(n: usize) -> Self {
        Self::new(n, vec![1; n])
    }

    /// Deal a real crypto transcript for the given validator index.
    pub fn deal_transcript(
        &self,
        validator_index: usize,
    ) -> (ChunkyDKGTranscript, ChunkyTranscript) {
        let mut rng = StdRng::from_rng(thread_rng()).unwrap();
        let input_secret = ChunkyInputSecret::generate(&mut rng);
        let dealer = Player {
            id: validator_index,
        };

        let trx = ChunkyDKG::deal(
            &self.dkg_config,
            &self.private_keys[validator_index],
            &self.public_keys[validator_index],
            &input_secret,
            &self.session_metadata,
            &dealer,
            &mut rng,
        );

        let dkg_transcript = ChunkyDKGTranscript::new(
            999,
            self.addrs[validator_index],
            bcs::to_bytes(&trx).unwrap(),
        );

        (dkg_transcript, trx)
    }

    /// Deal transcripts for the given indices, aggregate their subtranscripts,
    /// and return the AggregatedSubtranscript.
    pub fn aggregate_subtranscripts(&self, indices: &[usize]) -> AggregatedSubtranscript {
        let subtranscripts: Vec<_> = indices
            .iter()
            .map(|&i| {
                let (_, trx) = self.deal_transcript(i);
                trx.get_subtranscript()
            })
            .collect();

        use aptos_dkg::pvss::traits::transcript::Aggregatable;
        let agg =
            Aggregatable::aggregate(&self.dkg_config.threshold_config, subtranscripts).unwrap();

        let mut sorted_indices: Vec<usize> = indices.to_vec();
        sorted_indices.sort();
        // Map indices through address sort order to match production code behavior.
        // Production code sorts contributors by AccountAddress, then maps to Player indices.
        // We must do the same.
        let mut contributor_addrs: Vec<AccountAddress> =
            indices.iter().map(|&i| self.addrs[i]).collect();
        contributor_addrs.sort();
        let addr_to_index = self
            .epoch_state
            .verifier
            .address_to_validator_index()
            .clone();
        let dealers: Vec<Player> = contributor_addrs
            .into_iter()
            .map(|addr| Player {
                id: *addr_to_index.get(&addr).unwrap(),
            })
            .collect();

        AggregatedSubtranscript {
            subtranscript: agg,
            dealers,
        }
    }

    pub fn spks(&self) -> Vec<DealerPublicKey> {
        self.public_keys.clone()
    }
}

pub struct DummyNetworkSender;

#[async_trait]
impl RBNetworkSender<DKGMessage> for DummyNetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        _receiver: AccountAddress,
        _raw_message: Bytes,
        _timeout: Duration,
    ) -> anyhow::Result<DKGMessage> {
        anyhow::bail!("dummy sender")
    }

    async fn send_rb_rpc(
        &self,
        author: AccountAddress,
        _message: DKGMessage,
        timeout: Duration,
    ) -> anyhow::Result<DKGMessage> {
        self.send_rb_rpc_raw(author, Bytes::new(), timeout).await
    }

    fn to_bytes_by_protocol(
        &self,
        _peers: Vec<AccountAddress>,
        _message: DKGMessage,
    ) -> anyhow::Result<HashMap<AccountAddress, Bytes>> {
        Ok(HashMap::new())
    }

    fn sort_peers_by_latency(&self, _: &mut [AccountAddress]) {}
}
