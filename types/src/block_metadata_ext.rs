// Copyright © Aptos Foundation

use serde::{Deserialize, Serialize};
use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress;
use move_core_types::value::MoveValue;
use crate::dkg::DKGTranscriptWrapper;
use crate::randomness::Randomness;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockMetadataExt {
    V2(BlockMetadataExtV2),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMetadataExtV2 {
    id: HashValue,
    epoch: u64,
    round: u64,
    proposer: AccountAddress,
    #[serde(with = "serde_bytes")]
    previous_block_votes_bitvec: Vec<u8>,
    failed_proposer_indices: Vec<u32>,
    timestamp_usecs: u64,
    dkg_transcript: Option<DKGTranscriptWrapper>,
    randomness: Option<Randomness>,
}

impl BlockMetadataExt {
    pub fn new_v2(
        id: HashValue,
        epoch: u64,
        round: u64,
        proposer: AccountAddress,
        previous_block_votes_bitvec: Vec<u8>,
        failed_proposer_indices: Vec<u32>,
        timestamp_usecs: u64,
        dkg_transcript: Option<DKGTranscriptWrapper>,
        randomness: Option<Randomness>,
    ) -> Self {
        Self::V2(BlockMetadataExtV2 {
            id,
            epoch,
            round,
            proposer,
            previous_block_votes_bitvec,
            failed_proposer_indices,
            timestamp_usecs,
            dkg_transcript,
            randomness,
        })
    }

    pub fn id(&self) -> HashValue {
        match self {
            BlockMetadataExt::V2(obj) => obj.id
        }
    }

    pub fn get_prologue_ext_move_args(self, signer: AccountAddress) -> Vec<MoveValue> {
        let mut ret = vec![
            MoveValue::Signer(signer),
            MoveValue::Address(AccountAddress::from_bytes(self.id().to_vec()).unwrap()),
            MoveValue::U64(self.epoch()),
            MoveValue::U64(self.round()),
            MoveValue::Address(self.proposer()),
            MoveValue::Vector(
                self.failed_proposer_indices()
                    .iter()
                    .map(|x| MoveValue::U64((*x) as u64))
                    .collect(),
            ),
            MoveValue::Vector(
                self.previous_block_votes_bitvec()
                    .iter()
                    .map(|x|MoveValue::U8(*x))
                    .collect(),
            ),
            MoveValue::U64(self.timestamp_usecs()),
        ];

        match self.dkg_transcript() {
            None => {
                ret.push(MoveValue::Bool(false));
                ret.push(MoveValue::Vector(vec![]));
            }
            Some(trx) => {
                let move_bytes = bcs::to_bytes(trx).unwrap()
                    .into_iter()
                    .map(MoveValue::U8)
                    .collect();
                ret.push(MoveValue::Bool(true));
                ret.push(MoveValue::Vector(move_bytes));
            }
        }

        match self.randomness() {
            None => {
                ret.push(MoveValue::Bool(false));
                ret.push(MoveValue::Vector(vec![]));
            }
            Some(randomness) => {
                let move_bytes = randomness.randomness().to_vec().into_iter().map(MoveValue::U8).collect();
                ret.push(MoveValue::Bool(true));
                ret.push(MoveValue::Vector(move_bytes));
            }
        }
        ret
    }

    pub fn timestamp_usecs(&self) -> u64 {
        match self {
            BlockMetadataExt::V2(obj) => obj.timestamp_usecs
        }
    }

    pub fn proposer(&self) -> AccountAddress {
        match self {
            BlockMetadataExt::V2(obj) => obj.proposer
        }
    }

    pub fn previous_block_votes_bitvec(&self) -> &Vec<u8> {
        match self {
            BlockMetadataExt::V2(obj) => &obj.previous_block_votes_bitvec
        }
    }

    pub fn failed_proposer_indices(&self) -> &Vec<u32> {
        match self {
            BlockMetadataExt::V2(obj) => &obj.failed_proposer_indices
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            BlockMetadataExt::V2(obj) => obj.epoch
        }
    }

    pub fn round(&self) -> u64 {
        match self {
            BlockMetadataExt::V2(obj) => obj.round
        }
    }

    pub fn dkg_transcript(&self) -> &Option<DKGTranscriptWrapper> {
        match self {
            BlockMetadataExt::V2(obj) => &obj.dkg_transcript
        }
    }

    pub fn randomness(&self) -> &Option<Randomness> {
        match self {
            BlockMetadataExt::V2(obj) => &obj.randomness
        }
    }
}
