// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::api::{MovementAptosRestClient, MovementRestClient};
use clap::Parser;

#[derive(Parser)]
#[clap(
    name = "migration-api-validation",
    about = "Validates api conformity after movement migration."
)]
pub struct Command {
    #[clap(long = "movement", help = "The url of the Movement REST endpoint.")]
    pub movement_rest_api_url: String,
    #[clap(value_parser)]
    #[clap(
        long = "movement-aptos",
        help = "The url of the Movement Aptos REST endpoint."
    )]
    pub movement_aptos_rest_api_url: String,
}

impl Command {
    pub async fn run(self) -> anyhow::Result<()> {
        let _movement_rest_client = MovementRestClient::new(&self.movement_rest_api_url)?;
        let _movement_aptos_rest_client =
            MovementAptosRestClient::new(&self.movement_aptos_rest_api_url)?;

        Ok(())
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Command::command().debug_assert()
}
