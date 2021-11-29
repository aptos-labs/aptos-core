// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account,
    context::UserContext,
    deploy,
    dev_api_client::DevApiClient,
    shared::{self, normalized_network_name, Home, Network},
};
use anyhow::{anyhow, Result};
use core::convert::TryFrom;
use diem_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use diem_sdk::{
    client::AccountAddress, transaction_builder::TransactionFactory, types::LocalAccount,
};
use diem_types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey};
use move_cli::package::cli::{self, UnitTestResult};
use move_package::BuildConfig;
use move_unit_test::UnitTestingConfig;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};
use structopt::StructOpt;
use tempfile::TempDir;
use url::Url;

pub async fn run_e2e_tests(
    home: &Home,
    project_path: &Path,
    network: Network,
) -> Result<ExitStatus> {
    let _config = shared::read_project_config(project_path)?;

    println!("Connecting to {}...", network.get_json_rpc_url());
    let client = DevApiClient::new(reqwest::Client::new(), network.get_dev_api_url())?;
    let factory = TransactionFactory::new(ChainId::test());

    let (private_key1, mut account1) =
        create_account(home.get_root_key_path(), &client, &factory).await?;

    // TODO: Because we both codegen and deploy::deploy, this code path results
    // in two move package compilation steps. Ideally, compilation would only
    // happen once, and the second redundant build would be skipped. At least
    // it's cached atm.
    shared::codegen_typescript_libraries(project_path, &account1.address())?;
    deploy::deploy(&client, &mut account1, project_path).await?;

    let tmp_dir = TempDir::new()?;
    let key1_path = tmp_dir.path().join("private1.key");
    generate_key::save_key(private_key1, &key1_path);
    let latest_user = UserContext::new("latest", account1.address(), &key1_path);

    let (private_key2, account2) =
        create_account(home.get_root_key_path(), &client, &factory).await?;
    let key2_path = tmp_dir.path().join("private2.key");
    let test_user = UserContext::new("test", account2.address(), &key2_path);
    generate_key::save_key(private_key2, &key2_path);

    run_deno_test(home, project_path, &network, &[&latest_user, &test_user])
}

async fn create_account(
    root_key_path: &Path,
    client: &DevApiClient,
    factory: &TransactionFactory,
) -> Result<(Ed25519PrivateKey, LocalAccount)> {
    let mut treasury_account = account::get_treasury_account(client, root_key_path).await?;
    let account_key = generate_key::generate_key();
    let public_key = account_key.public_key();
    let derived_address = AuthenticationKey::ed25519(&public_key).derived_address();
    let seq_num = client
        .get_account_sequence_number(derived_address)
        .await
        .unwrap_or(0);
    let dupe_key = Ed25519PrivateKey::try_from(account_key.to_bytes().as_ref())?;
    let new_account = LocalAccount::new(derived_address, dupe_key, seq_num);
    account::create_account_via_dev_api(&mut treasury_account, &new_account, factory, client)
        .await?;
    Ok((account_key, new_account))
}

pub fn run_deno_test(
    home: &Home,
    project_path: &Path,
    network: &Network,
    users: &[&UserContext],
) -> Result<ExitStatus> {
    let test_path = project_path.join("e2e");
    run_deno_test_at_path(home, project_path, network, users, &test_path)
}

pub fn run_deno_test_at_path(
    home: &Home,
    project_path: &Path,
    network: &Network,
    users: &[&UserContext],
    test_path: &Path,
) -> Result<ExitStatus> {
    let filtered_envs = shared::get_filtered_envs_for_deno(home, project_path, network, users)?;
    let env_names: String = filtered_envs
        .keys()
        .cloned()
        .collect::<Vec<String>>()
        .join(",");
    let status = Command::new("deno")
        .args([
            "test",
            "--unstable",
            test_path.to_string_lossy().as_ref(),
            format!("--allow-env={}", env_names).as_str(),
            "--allow-read",
            format!(
                "--allow-net={},{}",
                host_and_port(&network.get_dev_api_url())?,
                host_and_port(&network.get_json_rpc_url())?,
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

    // Default publishing address to a placeholder address for Move unit tests,
    // which do not run against a Node, but solely in the Move VM.
    let publishing_address = AccountAddress::from_hex_literal(shared::PLACEHOLDER_ADDRESS)?;
    cli::run_move_unit_tests(
        &project_path.join(shared::MAIN_PKG_PATH),
        generate_build_config_for_testing(&publishing_address)?,
        unit_test_config,
        diem_vm::natives::diem_natives(),
        false,
    )
}

fn generate_build_config_for_testing(publishing_address: &AccountAddress) -> Result<BuildConfig> {
    let mut additional_named_addresses = BTreeMap::new();
    additional_named_addresses.insert(shared::SENDER_ADDRESS_NAME.to_string(), *publishing_address);
    Ok(BuildConfig {
        dev_mode: true,
        test_mode: true,
        generate_abis: true,
        additional_named_addresses,
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
            let normalized_network = home
                .get_network_struct_from_toml(normalized_network_name(network.clone()).as_str())?;

            let unit_status = ExitStatus::from(run_move_unit_tests(normalized_path.as_path())?);
            let e2e_status =
                run_e2e_tests(home, normalized_path.as_path(), normalized_network).await?;

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
