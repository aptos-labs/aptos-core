// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    velor::move_test_helpers, smoke_test_environment::SwarmBuilder,
    utils::check_create_mint_transfer, workspace_builder, workspace_builder::workspace_root,
};
use velor_crypto::ValidCryptoMaterialStringExt;
use velor_forge::Swarm;
use velor_gas_algebra::GasQuantity;
use velor_gas_schedule::{VelorGasParameters, InitialGasSchedule, ToOnChainGasSchedule};
use velor_release_builder::{
    components::{
        feature_flags::{FeatureFlag, Features},
        framework::FrameworkReleaseConfig,
        gas::generate_gas_upgrade_proposal,
        ExecutionMode, GasScheduleLocator, Proposal, ProposalMetadata,
    },
    ReleaseEntry,
};
use velor_temppath::TempPath;
use velor_types::on_chain_config::{FeatureFlag as VelorFeatureFlag, OnChainConsensusConfig};
use move_binary_format::file_format_common::VERSION_DEFAULT_LANG_V2;
use std::{fs, path::PathBuf, process::Command, sync::Arc};

// Ignored. This is redundant with the forge compat test but this test is easier to run locally and
// could help debug much faster
#[ignore]
// TODO: currently fails when quorum store is enabled by hard-coding. Investigate why.
#[tokio::test]
/// This test verifies the flow of velor framework upgrade process.
/// i.e: The network will be alive after applying the new velor framework release.
async fn test_upgrade_flow() {
    // prebuild tools.
    let velor_cli = workspace_builder::get_bin("velor");

    let num_nodes = 5;
    let (mut env, _cli, _) = SwarmBuilder::new_local(num_nodes)
        .with_velor_testnet()
        .build_with_cli(0)
        .await;

    let url = env.velor_public_info().url().to_string();
    let private_key = env
        .velor_public_info()
        .root_account()
        .private_key()
        .to_encoded_string()
        .unwrap();

    // Bump the limit in gas schedule
    // TODO: Replace this logic with velor-gas
    let mut gas_parameters = VelorGasParameters::initial();
    gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);

    let gas_schedule = velor_types::on_chain_config::GasScheduleV2 {
        feature_version: velor_gas_schedule::LATEST_GAS_FEATURE_VERSION,
        entries: gas_parameters
            .to_on_chain_gas_schedule(velor_gas_schedule::LATEST_GAS_FEATURE_VERSION),
    };

    let (_, update_gas_script) =
        generate_gas_upgrade_proposal(None, &gas_schedule, true, None, false)
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
        .join("velor-move")
        .join("framework")
        .join("velor-framework");

    assert!(Command::new(velor_cli.as_path())
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
    env.velor_public_info()
        .root_account()
        .increment_sequence_number();

    let upgrade_scripts_folder = TempPath::new();
    upgrade_scripts_folder.create_as_dir().unwrap();

    let config = velor_release_builder::ReleaseConfig {
        name: "Default".to_string(),
        remote_endpoint: None,
        proposals: vec![
            Proposal {
                execution_mode: ExecutionMode::RootSigner,
                name: "framework".to_string(),
                metadata: ProposalMetadata::default(),
                update_sequence: vec![ReleaseEntry::Framework(FrameworkReleaseConfig {
                    bytecode_version: VERSION_DEFAULT_LANG_V2, // TODO: remove explicit bytecode version from sources
                    git_hash: None,
                })],
            },
            Proposal {
                execution_mode: ExecutionMode::RootSigner,
                name: "gas".to_string(),
                metadata: ProposalMetadata::default(),
                update_sequence: vec![ReleaseEntry::Gas {
                    old: None,
                    new: GasScheduleLocator::Current,
                }],
            },
            Proposal {
                execution_mode: ExecutionMode::RootSigner,
                name: "feature_flags".to_string(),
                metadata: ProposalMetadata::default(),
                update_sequence: vec![
                    ReleaseEntry::FeatureFlag(Features {
                        enabled: VelorFeatureFlag::default_features()
                            .into_iter()
                            .map(FeatureFlag::from)
                            .collect(),
                        disabled: vec![],
                    }),
                    ReleaseEntry::Consensus(OnChainConsensusConfig::default()),
                ],
            },
        ],
    };

    config
        .generate_release_proposal_scripts(upgrade_scripts_folder.path())
        .await
        .unwrap();
    let mut scripts = walkdir::WalkDir::new(upgrade_scripts_folder.path())
        .sort_by_file_name()
        .into_iter()
        .filter_map(|path| match path {
            Ok(path) => {
                if path.path().ends_with("move") {
                    Some(path.path().to_path_buf())
                } else {
                    None
                }
            },
            Err(_) => None,
        })
        .collect::<Vec<_>>();

    scripts.sort();

    let framework_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("velor-move")
        .join("framework")
        .join("velor-framework");

    for path in scripts.iter() {
        assert!(Command::new(velor_cli.as_path())
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

        env.velor_public_info()
            .root_account()
            .increment_sequence_number();
    }

    //TODO: Make sure gas schedule is indeed updated by the tool.

    // Test the module publishing workflow
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_path_v1 = base_dir.join("src/velor/package_publish_modules_v1/");

    move_test_helpers::publish_package(&mut env.velor_public_info(), base_path_v1)
        .await
        .unwrap();

    check_create_mint_transfer(&mut env).await;
}

// This test is intentionally disabled because it's taking ~500s to execute right now.
// The main reason is that compilation of scripts takes a bit too long, as the Move compiler will need
// to repeatedly compile all the velor framework pacakges as dependency
//
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn test_release_validate_tool_multi_step() {
    let (mut env, _, _) = SwarmBuilder::new_local(1)
        .with_init_genesis_stake(Arc::new(|_, genesis_stake_amount| {
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
    let config = velor_release_builder::ReleaseConfig::default();

    let root_key = TempPath::new();
    root_key.create_as_file().unwrap();
    let mut root_key_path = root_key.path().to_path_buf();
    root_key_path.set_extension("key");

    std::fs::write(
        root_key_path.as_path(),
        bcs::to_bytes(&env.chain_info().root_account().private_key()).unwrap(),
    )
    .unwrap();

    let network_config = velor_release_builder::validate::NetworkConfig {
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

    network_config.mint_to_validator(None).await.unwrap();

    velor_release_builder::validate::validate_config(config, network_config, None)
        .await
        .unwrap();

    let root_account = env.velor_public_info().root_account().address();
    // Test the module publishing workflow
    let sequence_number = env
        .velor_public_info()
        .client()
        .get_account(root_account)
        .await
        .unwrap()
        .inner()
        .sequence_number;
    env.velor_public_info()
        .root_account()
        .set_sequence_number(sequence_number);

    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_path_v1 = base_dir.join("src/velor/package_publish_modules_v1/");

    move_test_helpers::publish_package(&mut env.velor_public_info(), base_path_v1)
        .await
        .unwrap();

    check_create_mint_transfer(&mut env).await;
}
