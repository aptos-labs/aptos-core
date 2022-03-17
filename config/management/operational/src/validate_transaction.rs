// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{rest_client::RestClient, TransactionContext};
use aptos_management::{config::ConfigPath, error::Error};
use aptos_types::account_address::AccountAddress;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct ValidateTransaction {
    #[structopt(flatten)]
    config: ConfigPath,
    /// JSON-RPC Endpoint (e.g. http://localhost:8080)
    #[structopt(long, required_unless = "config")]
    json_server: Option<String>,
    #[structopt(long, help = "AccountAddress to check transactions")]
    account_address: AccountAddress,
    #[structopt(long, help = "Sequence number to verify")]
    sequence_number: u64,
}

/// Returns `true` if we've passed by the expected sequence number
impl ValidateTransaction {
    pub fn new(json_server: String, account_address: AccountAddress, sequence_number: u64) -> Self {
        Self {
            config: Default::default(),
            json_server: Some(json_server),
            account_address,
            sequence_number,
        }
    }

    pub async fn execute(&self) -> Result<TransactionContext, Error> {
        let config = self.config.load()?.override_json_server(&self.json_server);
        let vm_status = RestClient::new(config.json_server)
            .transaction_status(self.account_address, self.sequence_number)
            .await?;
        Ok(TransactionContext::new_with_validation(
            self.account_address,
            self.sequence_number,
            vm_status,
        ))
    }
}
