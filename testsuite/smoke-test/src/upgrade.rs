// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos::move_test_helpers, smoke_test_environment::SwarmBuilder,
    test_utils::check_create_mint_transfer, workspace_builder, workspace_builder::workspace_root,
};
use aptos_crypto::ValidCryptoMaterialStringExt;
use aptos_gas::{AptosGasParameters, GasQuantity, InitialGasSchedule, ToOnChainGasSchedule};
use aptos_release_builder::components::{
    feature_flags::{FeatureFlag, Features},
    gas::generate_gas_upgrade_proposal,
};
use aptos_temppath::TempPath;
use forge::Swarm;
use std::fs;
use std::process::Command;

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
        entries: gas_parameters.to_on_chain_gas_schedule(),
    };

    let (_, update_gas_script) = generate_gas_upgrade_proposal(&gas_schedule, true, "".to_owned())
        .unwrap()
        .pop()
        .unwrap();

    let gas_script_path = TempPath::new();
    let mut gas_script_path = gas_script_path.path().to_path_buf();
    gas_script_path.set_extension("move");
    fs::write(gas_script_path.as_path(), update_gas_script).unwrap();

    assert!(Command::new(aptos_cli.as_path())
        .current_dir(workspace_root())
        .args(&vec![
            "move",
            "run-script",
            "--script-path",
            gas_script_path.to_str().unwrap(),
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

    for path in scripts.iter() {
        assert!(Command::new(aptos_cli.as_path())
            .current_dir(workspace_root())
            .args(&vec![
                "move",
                "run-script",
                "--script-path",
                path.to_str().unwrap(),
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
