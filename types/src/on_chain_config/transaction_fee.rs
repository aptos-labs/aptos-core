// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct TransactionFeeBurnCap;

impl OnChainConfig for TransactionFeeBurnCap {
    const MODULE_IDENTIFIER: &'static str = "transaction_fee";
    const TYPE_IDENTIFIER: &'static str = "AptosCoinCapabilities";
}
