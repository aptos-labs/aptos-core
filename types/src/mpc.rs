// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use move_core_types::ident_str;
use move_core_types::identifier::IdentStr;
use move_core_types::language_storage::TypeTag;
use move_core_types::move_resource::MoveStructType;
use crate::on_chain_config::OnChainConfig;
use crate::move_any::{Any as MoveAny, AsMoveAny};
use crate::validator_verifier::ValidatorConsensusInfo;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TaskSpec {
    pub group_element: Vec<u8>,
    pub secret_idx: u64,
}


#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TaskState {
    pub task: TaskSpec,
    pub result: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SharedSecretState {
    pub transcript_for_cur_epoch: Option<Vec<u8>>,
    pub transcript_for_next_epoch: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MPCState {
    pub shared_secrets: Vec<SharedSecretState>,
    pub tasks: Vec<TaskState>,
}

impl OnChainConfig for MPCState {
    const MODULE_IDENTIFIER: &'static str = "mpc";
    const TYPE_IDENTIFIER: &'static str = "State";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MPCEventMoveStruct {
    variant: MoveAny,
}

impl MoveStructType for MPCEventMoveStruct {
    const MODULE_NAME: &'static IdentStr = ident_str!("mpc");
    const STRUCT_NAME: &'static IdentStr = ident_str!("MPCEvent");
}

pub static MPC_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(MPCEventMoveStruct::struct_tag())));

#[derive(Debug)]
pub enum MPCEvent {
    ReconfigStart(MPCEventReconfigStart),
    StateUpdated(MPCEventStateUpdated),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MPCEventReconfigStart {
    pub epoch: u64,
    pub next_validator_set: Vec<ValidatorConsensusInfo>,
}

impl AsMoveAny for MPCEventReconfigStart {
    const MOVE_TYPE_NAME: &'static str = "0x1::mpc::MPCEventReconfigStart";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MPCEventStateUpdated {
    pub epoch: u64,
    pub new_state: MPCState,
}

impl AsMoveAny for MPCEventStateUpdated {
    const MOVE_TYPE_NAME: &'static str = "0x1::mpc::MPCEventStateUpdated";
}


impl MPCEvent {
    pub fn try_from(on_chain: MPCEventMoveStruct) -> anyhow::Result<Self> {
        match on_chain.variant.type_name.as_str() {
            MPCEventReconfigStart::MOVE_TYPE_NAME => {
                let variant = MoveAny::unpack::<MPCEventReconfigStart>(MPCEventReconfigStart::MOVE_TYPE_NAME, on_chain.variant).unwrap();
                Ok(Self::ReconfigStart(variant))
            },
            MPCEventStateUpdated::MOVE_TYPE_NAME => {
                let variant = MoveAny::unpack::<MPCEventStateUpdated>(MPCEventStateUpdated::MOVE_TYPE_NAME, on_chain.variant).unwrap();
                Ok(Self::StateUpdated(variant))
            }
            _ => {
                Err(anyhow!("unknown MPCEvent variant"))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct TaskResult {
    pub task_idx: usize,
    pub raise_result: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct ReconfigWorkResult {
    pub next_transcript: Vec<u8>,
}
