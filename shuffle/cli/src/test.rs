// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account, deploy,
    shared::{self, MAIN_PKG_PATH},
};
use anyhow::{anyhow, Context, Result};
use diem_config::config::NodeConfig;
use diem_crypto::PrivateKey;
use diem_sdk::{
    client::{AccountAddress, BlockingClient},
    transaction_builder::TransactionFactory,
    types::LocalAccount,
};
use diem_types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey};
use move_cli::package::cli;
use move_package::BuildConfig;
use shared::Home;
use std::{
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};
use structopt::StructOpt;
use url::Url;

pub async fn run_e2e_tests(home: &Home, project_path: &Path, network: Url) -> Result<()> {
    let _config = shared::read_project_config(project_path)?;
    shared::generate_typescript_libraries(project_path)?;

    let config = NodeConfig::load(&home.get_validator_config_path()).with_context(|| {
        format!(
            "Failed to load NodeConfig from file: {:?}",
            home.get_validator_config_path()
        )
    })?;
    println!("Connecting to {}...", network.as_str());
    let client = BlockingClient::new(network.as_str());
    let factory = TransactionFactory::new(ChainId::test());

    let new_account = create_test_account(home, &client, &factory)?;
    create_receiver_account(home, &client, &factory)?;
    deploy::handle(home, project_path, network.clone()).await?;

    run_deno_test(
        home,
        project_path,
        &Url::from_str(config.json_rpc.address.to_string().as_str())?,
        &network,
        home.get_test_key_path(),
        new_account.address(),
    )
}

// Set up a new test account
fn create_test_account(
    home: &Home,
    client: &BlockingClient,
    factory: &TransactionFactory,
) -> Result<LocalAccount> {
    let mut treasury_account = account::get_treasury_account(client, home.get_root_key_path());
    // TODO: generate random key by using let new_account_key = generate_key::generate_key();
    let new_account_key = generate_key::load_key(home.get_latest_key_path());
    let public_key = new_account_key.public_key();
    let derived_address = AuthenticationKey::ed25519(&public_key).derived_address();
    let new_account = LocalAccount::new(derived_address, new_account_key, 0);
    account::create_account_onchain(&mut treasury_account, &new_account, factory, client)?;
    Ok(new_account)
}

// Set up a new test account
fn create_receiver_account(
    home: &Home,
    client: &BlockingClient,
    factory: &TransactionFactory,
) -> Result<LocalAccount> {
    let mut treasury_account = account::get_treasury_account(client, home.get_root_key_path());
    let receiver_account_key = generate_key::load_key(home.get_test_key_path());
    let public_key = receiver_account_key.public_key();
    let address = AuthenticationKey::ed25519(&public_key).derived_address();
    let receiver_account = LocalAccount::new(address, receiver_account_key, 0);
    account::create_account_onchain(&mut treasury_account, &receiver_account, factory, client)?;

    Ok(receiver_account)
}

// Run shuffle test using deno
pub fn run_deno_test(
    home: &Home,
    project_path: &Path,
    json_rpc_url: &Url,
    dev_api_url: &Url,
    key_path: &Path,
    sender_address: AccountAddress,
) -> Result<()> {
    let tests_path_string = project_path
        .join("e2e")
        .as_path()
        .to_string_lossy()
        .to_string();

    let filtered_envs = shared::get_filtered_envs_for_deno(
        home,
        project_path,
        dev_api_url,
        key_path,
        sender_address,
    );
    Command::new("deno")
        .args([
            "test",
            "--unstable",
            tests_path_string.as_str(),
            "--allow-env=PROJECT_PATH,SHUFFLE_HOME,SHUFFLE_NETWORK,PRIVATE_KEY_PATH,SENDER_ADDRESS",
            "--allow-read",
            format!(
                "--allow-net={},{}",
                host_and_port(dev_api_url)?,
                host_and_port(json_rpc_url)?,
            )
            .as_str(),
        ])
        .envs(&filtered_envs)
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
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

pub fn run_move_unit_tests(project_path: &Path) -> Result<()> {
    let unit_test_cmd = cli::PackageCommand::UnitTest {
        // Setting to default values with exception of report_storage_on_error
        // to get more information about a failed test
        instruction_execution_bound: 5000,
        filter: None,
        list: false,
        num_threads: 8,
        report_statistics: false,
        report_storage_on_error: true,
        check_stackless_vm: false,
        verbose_mode: false,
        compute_coverage: false,
    };

    cli::handle_package_commands(
        &Some(project_path.join(MAIN_PKG_PATH)),
        generate_build_config_for_testing()?,
        &unit_test_cmd,
        diem_vm::natives::diem_natives(),
    )
}

fn generate_build_config_for_testing() -> Result<BuildConfig> {
    Ok(BuildConfig {
        dev_mode: true,
        test_mode: true,
        generate_docs: false,
        generate_abis: true,
        install_dir: None,
        force_recompilation: false,
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
    match cmd {
        TestCommand::E2e {
            project_path,
            network,
        } => {
            run_e2e_tests(
                home,
                shared::normalized_project_path(project_path)?.as_path(),
                shared::normalized_network(home, network)?,
            )
            .await
        }
        TestCommand::Unit { project_path } => {
            run_move_unit_tests(shared::normalized_project_path(project_path)?.as_path())
        }
        TestCommand::All {
            project_path,
            network,
        } => {
            let normalized_path = shared::normalized_project_path(project_path)?;
            run_move_unit_tests(normalized_path.as_path())?;
            run_e2e_tests(
                home,
                normalized_path.as_path(),
                shared::normalized_network(home, network)?,
            )
            .await
        }
    }
}
