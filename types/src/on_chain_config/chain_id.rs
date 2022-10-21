// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// Defines the version of Aptos Validator software.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ChainId {
    pub id: u8,
}

impl OnChainConfig for ChainId {
    const MODULE_IDENTIFIER: &'static str = "chain_id";
    const TYPE_IDENTIFIER: &'static str = "ChainId";
}
