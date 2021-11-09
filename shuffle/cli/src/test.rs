// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account, deploy,
    shared::{self, normalized_network_name, Home, Network, NetworkHome, MAIN_PKG_PATH},
};
use anyhow::{anyhow, Result};
use diem_crypto::PrivateKey;
use diem_sdk::{
    client::{AccountAddress, BlockingClient},
    transaction_builder::TransactionFactory,
    types::LocalAccount,
};
use diem_types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey};
use move_cli::package::cli::{self, UnitTestResult};
use move_package::BuildConfig;
use move_unit_test::UnitTestingConfig;
use std::{
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};
use structopt::StructOpt;
use url::Url;

pub async fn run_e2e_tests(
    home: &Home,
    project_path: &Path,
    network: Network,
) -> Result<ExitStatus> {
    let network_home = NetworkHome::new(
        home.get_networks_path()
            .join(shared::LOCALHOST_NAME)
            .as_path(),
    );
    let _config = shared::read_project_config(project_path)?;
    shared::generate_typescript_libraries(project_path)?;

    println!("Connecting to {}...", network.get_json_rpc_url()?);
    let client = BlockingClient::new(network.get_json_rpc_url()?.as_str());
    let factory = TransactionFactory::new(ChainId::test());

    let test_account = create_account(
        home.get_root_key_path(),
        network_home.get_test_key_path(),
        &client,
        &factory,
    )?;
    let _receiver_account = create_account(
        home.get_root_key_path(),
        network_home.get_test_key_path(), // TODO: update to a different key to sender
        &client,
        &factory,
    )?;
    deploy::handle(&network_home, project_path, network.get_dev_api_url()?).await?;

    run_deno_test(
        home,
        project_path,
        &network,
        network_home.get_test_key_path(),
        test_account.address(),
    )
}

fn create_account(
    root_key_path: &Path,
    account_key_path: &Path,
    client: &BlockingClient,
    factory: &TransactionFactory,
) -> Result<LocalAccount> {
    let mut treasury_account = account::get_treasury_account(client, root_key_path)?;
    // TODO: generate random key by using let account_key = generate_key::generate_key();
    let account_key = generate_key::load_key(account_key_path);
    let public_key = account_key.public_key();
    let derived_address = AuthenticationKey::ed25519(&public_key).derived_address();
    let new_account = LocalAccount::new(derived_address, account_key, 0);
    account::create_local_account(&mut treasury_account, &new_account, factory, client)?;
    Ok(new_account)
}

pub fn run_deno_test(
    home: &Home,
    project_path: &Path,
    network: &Network,
    key_path: &Path,
    sender_address: AccountAddress,
) -> Result<ExitStatus> {
    let test_path = project_path.join("e2e");
    run_deno_test_at_path(
        home,
        project_path,
        network,
        key_path,
        sender_address,
        &test_path,
    )
}

pub fn run_deno_test_at_path(
    home: &Home,
    project_path: &Path,
    network: &Network,
    key_path: &Path,
    sender_address: AccountAddress,
    test_path: &Path,
) -> Result<ExitStatus> {
    let filtered_envs =
        shared::get_filtered_envs_for_deno(home, project_path, network, key_path, sender_address)?;
    let status = Command::new("deno")
        .args([
            "test",
            "--unstable",
            test_path.to_string_lossy().as_ref(),
            "--allow-env=PROJECT_PATH,SHUFFLE_BASE_NETWORKS_PATH,SHUFFLE_NETWORK_NAME,SHUFFLE_NETWORK_DEV_API_URL,PRIVATE_KEY_PATH,SENDER_ADDRESS",
            "--allow-read",
            format!(
                "--allow-net={},{}",
                host_and_port(&network.get_dev_api_url()?)?,
                host_and_port(&network.get_json_rpc_url()?)?,
            )
            .as_str(),
        ])
        .envs(&filtered_envs)
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(status)
}

fn host_and_port(url: &Url) -> Result<String> {
    Ok(format!(
        "{}:{}",
        url.host_str()
            .ok_or_else(|| anyhow!("url should have domain host"))?,
        url.port_or_known_default()
            .ok_or_else(|| anyhow!("url should have port or default"))?,
    ))
}

pub fn run_move_unit_tests(project_path: &Path) -> Result<UnitTestResult> {
    let unit_test_config = UnitTestingConfig {
        report_storage_on_error: true,
        ..UnitTestingConfig::default_with_bound(None)
    };

    cli::run_move_unit_tests(
        &project_path.join(MAIN_PKG_PATH),
        generate_build_config_for_testing()?,
        unit_test_config,
        diem_vm::natives::diem_natives(),
        false,
    )
}

fn generate_build_config_for_testing() -> Result<BuildConfig> {
    Ok(BuildConfig {
        dev_mode: true,
        test_mode: true,
        generate_abis: true,
        ..Default::default()
    })
}

#[derive(Debug, StructOpt)]
pub enum TestCommand {
    #[structopt(about = "Runs end to end test in shuffle")]
    E2e {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,

        #[structopt(short, long)]
        network: Option<String>,
    },

    #[structopt(about = "Runs move move unit tests in project folder")]
    Unit {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
    },

    #[structopt(
        about = "Runs both end to end test in shuffle and move move unit tests in project folder"
    )]
    All {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,

        #[structopt(short, long)]
        network: Option<String>,
    },
}

pub async fn handle(home: &Home, cmd: TestCommand) -> Result<()> {
    let exit_status = match cmd {
        TestCommand::E2e {
            project_path,
            network,
        } => {
            run_e2e_tests(
                home,
                shared::normalized_project_path(project_path)?.as_path(),
                home.get_network_struct_from_toml(
                    normalized_network_name(network.clone()).as_str(),
                )?,
            )
            .await?
        }
        TestCommand::Unit { project_path } => ExitStatus::from(run_move_unit_tests(
            shared::normalized_project_path(project_path)?.as_path(),
        )?),
        TestCommand::All {
            project_path,
            network,
        } => {
            let normalized_path = shared::normalized_project_path(project_path)?;
            let unit_status = ExitStatus::from(run_move_unit_tests(normalized_path.as_path())?);
            let e2e_status = run_e2e_tests(
                home,
                normalized_path.as_path(),
                home.get_network_struct_from_toml(
                    normalized_network_name(network.clone()).as_str(),
                )?,
            )
            .await?;

            // prioritize returning failures
            if !unit_status.success() {
                unit_status
            } else {
                e2e_status
            }
        }
    };

    std::process::exit(exit_status.code().unwrap_or(1));
}
