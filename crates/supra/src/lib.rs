// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

// Copyright © Entropy Foundation

use anyhow::Result;

use aptos_types::transaction::TransactionPayload;

/// Arguments required by supra cli for its operation.
pub struct SupraCommandArguments {
    pub payload: TransactionPayload,
    pub profile: Option<String>,
    pub rpc: Option<reqwest::Url>,
    pub gas_unit_price: Option<u64>,
    pub max_gas: Option<u64>,
    pub expiration_secs: u64,
}

/// Trait required by supra cli for its operation.
pub trait SupraCommand {

    /// consume self and returns [SupraCommandArguments]
    fn supra_command_arguments(self) -> Result<SupraCommandArguments>;
}
