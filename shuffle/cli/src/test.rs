// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account, deploy,
    shared::{self, MAIN_PKG_PATH},
};
use anyhow::{anyhow, Context, Result};
use diem_config::config::{NodeConfig, DEFAULT_PORT};
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
    collections::HashMap,
    panic,
    path::{Path, PathBuf},
    process::Command,
};
use structopt::StructOpt;

pub fn run_e2e_tests(project_path: &Path) -> Result<()> {
    let _config = shared::read_config(project_path)?;
    shared::generate_typescript_libraries(project_path)?;
    let home = Home::new(shared::get_home_path().as_path())?;

    let config = NodeConfig::load(&home.get_validator_config_path()).with_context(|| {
        format!(
            "Failed to load NodeConfig from file: {:?}",
            home.get_validator_config_path()
        )
    })?;
    let json_rpc_url = format!("http://0.0.0.0:{}", config.json_rpc.address.port());
    let network = json_rpc_url.as_str();
    println!("Connecting to {}...", network);
    let client = BlockingClient::new(network);
    let factory = TransactionFactory::new(ChainId::test());

    let mut new_account = create_test_account(&client, &home, &factory)?;
    create_receiver_account(&client, &home, &factory)?;
    send_module_transaction(&client, &mut new_account, project_path, &factory)?;
    run_deno_test(
        project_path,
        &config,
        network,
        home.get_test_key_path(),
        new_account.address(),
    )
}

// Set up a new test account
fn create_test_account(
    client: &BlockingClient,
    home: &Home,
    factory: &TransactionFactory,
) -> Result<LocalAccount> {
    let mut root_account = account::get_root_account(client, home.get_root_key_path());
    // TODO: generate random key by using let new_account_key = generate_key::generate_key();
    let new_account_key = generate_key::load_key(home.get_latest_key_path());
    let public_key = new_account_key.public_key();
    let derived_address = AuthenticationKey::ed25519(&public_key).derived_address();
    let new_account = LocalAccount::new(derived_address, new_account_key, 0);
    account::create_account_onchain(&mut root_account, &new_account, factory, client)?;
    Ok(new_account)
}

// Set up a new test account
fn create_receiver_account(
    client: &BlockingClient,
    home: &Home,
    factory: &TransactionFactory,
) -> Result<LocalAccount> {
    let mut root_account = account::get_root_account(client, home.get_root_key_path());
    let receiver_account_key = generate_key::load_key(home.get_test_key_path());
    let public_key = receiver_account_key.public_key();
    let address = AuthenticationKey::ed25519(&public_key).derived_address();
    let receiver_account = LocalAccount::new(address, receiver_account_key, 0);
    account::create_account_onchain(&mut root_account, &receiver_account, factory, client)?;

    Ok(receiver_account)
}

// Publish user made module onchain
fn send_module_transaction(
    client: &BlockingClient,
    new_account: &mut LocalAccount,
    project_path: &Path,
    factory: &TransactionFactory,
) -> Result<()> {
    let account_seq_num = client
        .get_account(new_account.address())?
        .into_inner()
        .unwrap()
        .sequence_number;
    *new_account.sequence_number_mut() = account_seq_num;
    println!(
        "Deploy move module in {} ----------",
        project_path.to_string_lossy().to_string()
    );

    let compiled_package = shared::build_move_package(project_path.join(MAIN_PKG_PATH).as_ref())?;
    deploy::send_module_transaction(&compiled_package, client, new_account, factory)?;
    deploy::check_module_exists(client, new_account)
}

// Run shuffle test using deno
fn run_deno_test(
    project_path: &Path,
    config: &NodeConfig,
    network: &str,
    key_path: &Path,
    sender_address: AccountAddress,
) -> Result<()> {
    let tests_path_string = project_path
        .join("e2e")
        .as_path()
        .to_string_lossy()
        .to_string();

    let mut filtered_envs: HashMap<String, String> = HashMap::new();
    filtered_envs.insert(
        String::from("PROJECT_PATH"),
        project_path.to_str().unwrap().to_string(),
    );
    filtered_envs.insert(
        String::from("SHUFFLE_HOME"),
        shared::get_shuffle_dir().to_str().unwrap().to_string(),
    );

    filtered_envs.insert(String::from("SENDER_ADDRESS"), sender_address.to_string());
    filtered_envs.insert(
        String::from("PRIVATE_KEY_PATH"),
        key_path.to_string_lossy().to_string(),
    );

    filtered_envs.insert(String::from("SHUFFLE_NETWORK"), network.to_string());

    Command::new("deno")
        .args([
            "test",
            "--unstable",
            tests_path_string.as_str(),
            "--allow-env=PROJECT_PATH,SHUFFLE_HOME,SHUFFLE_NETWORK,PRIVATE_KEY_PATH,SENDER_ADDRESS",
            "--allow-read",
            format!(
                "--allow-net=:{},:{}",
                DEFAULT_PORT,
                config.json_rpc.address.port()
            )
            .as_str(),
        ])
        .envs(&filtered_envs)
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}

fn run_move_unit_tests(project_path: &Path) -> Result<()> {
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
    };

    panic::set_hook(Box::new(|_info| {
        // Allows us to return the error below instead of panic!
        // todo: Remove all panic catching after @tzakian PR lands
    }));

    let result = panic::catch_unwind(|| {
        cli::handle_package_commands(
            &Some(project_path.join(MAIN_PKG_PATH)),
            generate_build_config_for_testing()?,
            &unit_test_cmd,
        )
    });

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("Run move package build in the main folder of the project directory, then rerun shuffle test [subcommand]")),
    }
}

fn generate_build_config_for_testing() -> Result<BuildConfig> {
    Ok(BuildConfig {
        dev_mode: true,
        test_mode: true,
        generate_docs: false,
        generate_abis: true,
        install_dir: None,
    })
}

#[derive(Debug, StructOpt)]
pub enum TestCommand {
    #[structopt(about = "Runs end to end test in shuffle")]
    E2e {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
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
    },
}

pub fn handle(cmd: TestCommand) -> Result<()> {
    match cmd {
        TestCommand::E2e { project_path } => {
            run_e2e_tests(shared::normalized_project_path(project_path)?.as_path())
        }
        TestCommand::Unit { project_path } => {
            run_move_unit_tests(shared::normalized_project_path(project_path)?.as_path())
        }
        TestCommand::All { project_path } => {
            let normalized_path = shared::normalized_project_path(project_path)?;
            run_move_unit_tests(normalized_path.as_path())?;
            run_e2e_tests(normalized_path.as_path())
        }
    }
}
