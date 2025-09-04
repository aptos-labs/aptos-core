// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use velor_logger::{debug, info};
use velor_rest_client::Client;
use velor_types::{account_address::AccountAddress, chain_id::ChainId};
use clap::Parser;
use reqwest::Url;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Args {
    /// This should include the port, e.g. http://127.0.0.1:8080
    #[clap(long)]
    api_url: Url,
}

// It isn't great to have all these tests together like this, but it's an okay
// start given we had nothing at all prior to this.
#[tokio::main]
async fn main() -> Result<()> {
    velor_logger::Logger::new().init();

    let args = Args::parse();
    debug!("Running with args: {:#?}", args);

    let client = Client::new(args.api_url);

    let address = AccountAddress::ONE;
    info!("Running all queries against account: {}", address);

    let results = client
        .get_account_resources(address)
        .await
        .context("Failed get_account_resources")?;
    info!(
        "Successfully retrieved {} account resources with JSON",
        results.inner().len()
    );

    let results = client
        .get_account_resources_bcs(address)
        .await
        .context("Failed get_account_resources_bcs")?;
    info!(
        "Successfully retrieved {} account resources with BCS",
        results.inner().len()
    );

    let results = client
        .get_account_modules(address)
        .await
        .context("Failed get_account_modules")?;
    info!(
        "Successfully retrieved {} account modules with JSON",
        results.inner().len()
    );

    let results = client
        .get_account_modules_bcs(address)
        .await
        .context("Failed get_account_modules_bcs")?;
    info!(
        "Successfully retrieved {} account modules with BCS",
        results.inner().len()
    );

    let resource = "0x1::chain_id::ChainId";

    client
        .get_account_resource(address, resource)
        .await
        .context("Failed get_account_resource")?;
    info!("Successfully retrieved resource {} with JSON", resource);

    let balance_from_new_api = client
        .get_account_balance(address, "0x1::velor_coin::VelorCoin")
        .await
        .context("Failed get_account_balance")?;
    info!(
        "Successfully retrieved balance {}",
        balance_from_new_api.inner()
    );

    let balance_from_old_api = client
        .view_apt_account_balance(address)
        .await
        .context("Failed view_apt_account_balance")?;
    info!(
        "Successfully retrieved balance {}",
        balance_from_old_api.inner()
    );
    assert_eq!(balance_from_new_api.inner(), balance_from_old_api.inner());

    client
        .get_account_resource_bcs::<ChainId>(address, resource)
        .await
        .context("Failed get_account_resource_bcs")?;
    info!("Successfully retrieved resource {} with BCS", resource);

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
