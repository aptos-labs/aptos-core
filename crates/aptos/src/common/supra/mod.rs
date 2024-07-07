// Copyright © Aptos Foundation

use crate::common::types::{CliTypedResult, TransactionOptions};
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
    fn supra_command_arguments(self) -> CliTypedResult<SupraCommandArguments>;
}
