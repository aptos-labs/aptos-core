// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos::move_test_helpers, aptos_cli::validator::init_validator_account,
    smoke_test_environment::SwarmBuilder, test_utils::check_create_mint_transfer,
    workspace_builder, workspace_builder::workspace_root,
};
use aptos::move_tool::ArgWithType;
use aptos_crypto::ValidCryptoMaterialStringExt;
use aptos_forge::Swarm;
use aptos_gas::{AptosGasParameters, GasQuantity, InitialGasSchedule, ToOnChainGasSchedule};
use aptos_keygen::KeyGen;
use aptos_release_builder::components::{
    feature_flags::{FeatureFlag, Features},
    gas::generate_gas_upgrade_proposal,
};
use aptos_temppath::TempPath;
use std::{fs, path::PathBuf, process::Command, sync::Arc, thread, time::Duration};

// TODO: currently fails when quorum store is enabled by hard-coding. Investigate why.
#[tokio::test]
/// This test verifies the flow of aptos framework upgrade process.
/// i.e: The network will be alive after applying the new aptos framework release.
async fn test_upgrade_flow() {
    // prebuild tools.
    let aptos_cli = workspace_builder::get_bin("aptos");

    let num_nodes = 5;
    let (mut env, _cli, _) = SwarmBuilder::new_local(num_nodes)
        .with_aptos_testnet()
        .build_with_cli(0)
        .await;

    let url = env.aptos_public_info().url().to_string();
    let private_key = env
        .aptos_public_info()
        .root_account()
        .private_key()
        .to_encoded_string()
        .unwrap();

    // Bump the limit in gas schedule
    // TODO: Replace this logic with aptos-gas
    let mut gas_parameters = AptosGasParameters::initial();
    gas_parameters.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);

    let gas_schedule = aptos_types::on_chain_config::GasScheduleV2 {
        feature_version: aptos_gas::LATEST_GAS_FEATURE_VERSION,
        entries: gas_parameters.to_on_chain_gas_schedule(aptos_gas::LATEST_GAS_FEATURE_VERSION),
    };

    let (_, update_gas_script) =
        generate_gas_upgrade_proposal(&gas_schedule, true, "".to_owned().into_bytes())
            .unwrap()
            .pop()
            .unwrap();

    let gas_script_path = TempPath::new();
    let mut gas_script_path = gas_script_path.path().to_path_buf();
    gas_script_path.set_extension("move");
    fs::write(gas_script_path.as_path(), update_gas_script).unwrap();

    let framework_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("aptos-move")
        .join("framework")
        .join("aptos-framework");

    assert!(Command::new(aptos_cli.as_path())
        .current_dir(workspace_root())
        .args(&vec![
            "move",
            "run-script",
            "--script-path",
            gas_script_path.to_str().unwrap(),
            "--framework-local-dir",
            framework_path.as_os_str().to_str().unwrap(),
            "--sender-account",
            "0xA550C18",
            "--url",
            url.as_str(),
            "--private-key",
            private_key.as_str(),
            "--assume-yes",
        ])
        .output()
        .unwrap()
        .status
        .success());
    *env.aptos_public_info().root_account().sequence_number_mut() += 1;

    let upgrade_scripts_folder = TempPath::new();
    upgrade_scripts_folder.create_as_dir().unwrap();

    let config = aptos_release_builder::ReleaseConfig {
        feature_flags: Some(Features {
            enabled: vec![
                FeatureFlag::CodeDependencyCheck,
                FeatureFlag::TreatFriendAsPrivate,
            ],
            disabled: vec![],
        }),
        ..Default::default()
    };

    config
        .generate_release_proposal_scripts(upgrade_scripts_folder.path())
        .unwrap();
    let mut scripts = fs::read_dir(upgrade_scripts_folder.path())
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect::<Vec<_>>();

    scripts.sort();

    let framework_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("aptos-move")
        .join("framework")
        .join("aptos-framework");

    for path in scripts.iter() {
        assert!(Command::new(aptos_cli.as_path())
            .current_dir(workspace_root())
            .args(&vec![
                "move",
                "run-script",
                "--script-path",
                path.to_str().unwrap(),
                "--framework-local-dir",
                framework_path.as_os_str().to_str().unwrap(),
                "--sender-account",
                "0xA550C18",
                "--url",
                url.as_str(),
                "--private-key",
                private_key.as_str(),
                "--assume-yes",
            ])
            .output()
            .unwrap()
            .status
            .success());

        *env.aptos_public_info().root_account().sequence_number_mut() += 1;
    }

    //TODO: Make sure gas schedule is indeed updated by the tool.

    // Test the module publishing workflow
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_path_v1 = base_dir.join("src/aptos/package_publish_modules_v1/");

    move_test_helpers::publish_package(&mut env.aptos_public_info(), base_path_v1)
        .await
        .unwrap();

    check_create_mint_transfer(&mut env).await;
}

#[tokio::test]
async fn test_upgrade_flow_multi_step() {
    let (mut env, mut cli, _) = SwarmBuilder::new_local(1)
        .with_init_config(Arc::new(|_, _, genesis_stake_amount| {
            // make sure we have quorum
            *genesis_stake_amount = 2000000000000000;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.voting_duration_secs = 30;
            genesis_config.voting_power_increase_limit = 50;
            genesis_config.epoch_duration_secs = 4;
        }))
        .build_with_cli(2)
        .await;

    let upgrade_scripts_folder = TempPath::new();
    upgrade_scripts_folder.create_as_dir().unwrap();

    let config = aptos_release_builder::ReleaseConfig {
        feature_flags: Some(Features {
            enabled: vec![
                FeatureFlag::CodeDependencyCheck,
                FeatureFlag::TreatFriendAsPrivate,
            ],
            disabled: vec![],
        }),
        is_multi_step: true,
        ..Default::default()
    };

    config
        .generate_release_proposal_scripts(upgrade_scripts_folder.path())
        .unwrap();
    let mut scripts = fs::read_dir(upgrade_scripts_folder.path())
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect::<Vec<_>>();

    scripts.sort();

    // Create a proposal and vote for it to pass.
    let mut i = 0;
    while i < 2 {
        let pool_address = cli.account_id(i);
        cli.fund_account(i, Some(1000000000000000)).await.unwrap();

        let mut keygen = KeyGen::from_os_rng();
        let (validator_cli_index, _) =
            init_validator_account(&mut cli, &mut keygen, Some(1000000000000000)).await;

        cli.initialize_stake_owner(
            i,
            1000000000000000,
            Some(validator_cli_index),
            Some(validator_cli_index),
        )
        .await
        .unwrap();

        cli.increase_lockup(i).await.unwrap();

        if i == 0 {
            let first_script_path = PathBuf::from(scripts.get(0).unwrap());
            cli.create_proposal(
                validator_cli_index,
                "https://raw.githubusercontent.com/aptos-labs/aptos-core/b4fb9acfc297327c43d030def2b59037c4376611/testsuite/smoke-test/src/upgrade_multi_step_test_metadata.txt",
                first_script_path,
                pool_address,
                true,
            ).await.unwrap();
        };
        cli.vote(validator_cli_index, 0, true, false, vec![pool_address])
            .await;
        i += 1;
    }

    // Sleep to pass voting_duration_secs
    thread::sleep(Duration::from_secs(30));

    let mut first_pass = true;
    for path in scripts.iter() {
        let verify_proposal_response = cli
            .verify_proposal(0, path.to_str().unwrap())
            .await
            .unwrap();

        assert!(verify_proposal_response.verified);

        if first_pass {
            // we don't necessarily have the hash in `aptos_governance::ApprovedExecutionHashes`
            // in the first pass if we don't manually call add_approved_script_hash_script()
            first_pass = false;
        } else {
            let approved_execution_hash = env
                .aptos_public_info()
                .get_approved_execution_hash_at_aptos_governance(0)
                .await;
            assert_eq!(
                verify_proposal_response.computed_hash,
                hex::encode(approved_execution_hash)
            );
        };

        let args: Vec<ArgWithType> = vec![ArgWithType::u64(0)];
        cli.run_script_with_script_path(3, path.to_str().unwrap(), args, Vec::new())
            .await
            .unwrap();
    }

    // Test the module publishing workflow
    *env.aptos_public_info().root_account().sequence_number_mut() = 6;
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_path_v1 = base_dir.join("src/aptos/package_publish_modules_v1/");

    move_test_helpers::publish_package(&mut env.aptos_public_info(), base_path_v1)
        .await
        .unwrap();

    check_create_mint_transfer(&mut env).await;
}

// This test is intentionally disabled because it's taking ~500s to execute right now.
// The main reason is that compilation of scripts takes a bit too long, as the Move compiler will need
// to repeatedly compile all the aptos framework pacakges as dependency
//
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn test_release_validate_tool_multi_step() {
    let (mut env, _, _) = SwarmBuilder::new_local(1)
        .with_init_config(Arc::new(|_, _, genesis_stake_amount| {
            // make sure we have quorum
            *genesis_stake_amount = 2000000000000000;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.voting_duration_secs = 30;
            genesis_config.voting_power_increase_limit = 50;
            genesis_config.epoch_duration_secs = 4;
        }))
        .build_with_cli(2)
        .await;
    let config = aptos_release_builder::ReleaseConfig {
        is_multi_step: true,
        ..Default::default()
    };

    let root_key = TempPath::new();
    root_key.create_as_file().unwrap();
    let mut root_key_path = root_key.path().to_path_buf();
    root_key_path.set_extension("key");

    std::fs::write(
        root_key_path.as_path(),
        bcs::to_bytes(&env.chain_info().root_account().private_key()).unwrap(),
    )
    .unwrap();

    let network_config = aptos_release_builder::validate::NetworkConfig {
        endpoint: url::Url::parse(&env.chain_info().rest_api_url).unwrap(),
        root_key_path,
        validator_account: env.validators().last().unwrap().peer_id(),
        validator_key: env
            .validators()
            .last()
            .unwrap()
            .account_private_key()
            .as_ref()
            .unwrap()
            .private_key(),
        framework_git_rev: None,
    };

    aptos_release_builder::validate::validate_config(config, network_config)
        .await
        .unwrap();

    let root_account = env.aptos_public_info().root_account().address();
    // Test the module publishing workflow
    *env.aptos_public_info().root_account().sequence_number_mut() = env
        .aptos_public_info()
        .client()
        .get_account(root_account)
        .await
        .unwrap()
        .inner()
        .sequence_number;

    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_path_v1 = base_dir.join("src/aptos/package_publish_modules_v1/");

    move_test_helpers::publish_package(&mut env.aptos_public_info(), base_path_v1)
        .await
        .unwrap();

    check_create_mint_transfer(&mut env).await;
}
