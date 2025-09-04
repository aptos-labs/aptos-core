// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct TransactionFeeBurnCap;

impl OnChainConfig for TransactionFeeBurnCap {
    const MODULE_IDENTIFIER: &'static str = "transaction_fee";
    const TYPE_IDENTIFIER: &'static str = "VelorCoinCapabilities";
}
