// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::checks::api::active_feature_flags::GlobalFeatureCheck;
use crate::types::api::MovementAptosRestClient;
use clap::Parser;

mod active_feature_flags;

#[derive(Parser)]
#[clap(
    name = "migration-api-validation",
    about = "Validates api conformity after movement migration."
)]
pub struct Command {
    // #[clap(long = "movement", help = "The url of the Movement REST endpoint.")]
    // pub movement_rest_api_url: String,
    #[clap(value_parser)]
    #[clap(
        long = "movement-aptos",
        help = "The url of the Movement Aptos REST endpoint."
    )]
    pub movement_aptos_rest_api_url: String,
}

impl Command {
    pub async fn run(self) -> anyhow::Result<()> {
        // let _movement_rest_client = MovementRestClient::new(&self.movement_rest_api_url)?;
        let movement_aptos_rest_client =
            MovementAptosRestClient::new(&self.movement_aptos_rest_api_url)?;

        GlobalFeatureCheck::satisfies(&movement_aptos_rest_client).await?;

        Ok(())
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Command::command().debug_assert()
}
