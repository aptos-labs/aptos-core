// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use crate::on_chain_config::OnChainConfig;
use crate::move_any::Any as MoveAny;

#[derive(Deserialize, Serialize)]
pub struct TaskSpec {
    pub variant: MoveAny,
}

#[derive(Deserialize, Serialize)]
pub struct TaskState {
    pub task: TaskSpec,
    pub result: Option<Vec<u8>>,
}

#[derive(Deserialize, Serialize)]
pub struct SharedSecretState {
    pub transcript_for_cur_epoch: Option<Vec<u8>>,
    pub transcript_for_next_epoch: Option<Vec<u8>>,
}

#[derive(Deserialize, Serialize)]
pub struct MpcState {
    pub shared_secrets: Vec<SharedSecretState>,
    pub tasks: Vec<TaskState>,
}

impl OnChainConfig for MpcState {
    const MODULE_IDENTIFIER: &'static str = "mpc";
    const TYPE_IDENTIFIER: &'static str = "State";
}
