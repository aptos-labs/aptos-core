// Copyright © Aptos Foundation

// Copyright © Entropy Foundation

use anyhow::Result;
use aptos_types::transaction::TransactionPayload;

/// Arguments required by supra cli for its operation.
pub struct SupraCommandArguments {
    pub payload: TransactionPayload,
    pub profile: Option<String>,
    pub rpc: Option<reqwest::Url>,
}

/// Trait required by supra cli for its operation.
pub trait SupraCommand {

    /// consume self and returns [SupraCommandArguments]
    fn supra_command_arguments(self) -> Result<SupraCommandArguments>;
}
