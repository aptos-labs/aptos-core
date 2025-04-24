// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod dummy_provider;
mod jwk_consensus_basic;
mod jwk_consensus_per_issuer;
mod jwk_consensus_per_key;
mod jwk_consensus_provider_change_mind;

use crate::smoke_test_environment::SwarmBuilder;
use aptos::{common::types::TransactionSummary, test::CliTestFramework};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_types::{
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        unsupported::UnsupportedJWK,
        AllProvidersJWKs, PatchedJWKs, ProviderJWKs,
    },
    on_chain_config::OnChainJWKConsensusConfig,
};
use move_core_types::account_address::AccountAddress;
use std::time::Duration;

pub async fn update_jwk_consensus_config(
    cli: CliTestFramework,
    account_idx: usize,
    config: &OnChainJWKConsensusConfig,
) -> TransactionSummary {
    let script = match config {
        OnChainJWKConsensusConfig::Off => r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::jwk_consensus_config;
    fun main(core_resources: &signer) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let config = jwk_consensus_config::new_off();
        jwk_consensus_config::set_for_next_epoch(&framework, config);
        aptos_governance::reconfigure(&framework);
    }
}
"#
        .to_string(),
        OnChainJWKConsensusConfig::V1(config_v1) => {
            let provider_lines = config_v1
                .oidc_providers
                .iter()
                .map(|provider| {
                    format!(
                        "jwk_consensus_config::new_oidc_provider(utf8(b\"{}\"), utf8(b\"{}\")),",
                        provider.name, provider.config_url
                    )
                })
                .collect::<Vec<_>>()
                .join("\n            ");
            format!(
                r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::jwk_consensus_config;
    use std::string::utf8;

    fun main(core_resources: &signer) {{
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let config = jwk_consensus_config::new_v1(vector[
            {provider_lines}
        ]);
        jwk_consensus_config::set_for_next_epoch(&framework, config);
        aptos_governance::reconfigure(&framework);
    }}
}}
"#
            )
        },
    };
    println!("script={script}");

    cli.run_script(account_idx, script.as_str()).await.unwrap()
}

async fn get_patched_jwks(rest_client: &Client) -> PatchedJWKs {
    let maybe_response = rest_client
        .get_account_resource_bcs::<PatchedJWKs>(AccountAddress::ONE, "0x1::jwks::PatchedJWKs")
        .await;
    let response = maybe_response.unwrap();
    response.into_inner()
}

/// Patch the JWK with governance proposal and see it is effective.
#[tokio::test]
async fn jwk_patching() {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let client = swarm.validators().next().unwrap().rest_client();
    let root_idx = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(60))
        .await
        .expect("Epoch 2 taking too long to come!");

    info!("Insert a JWK.");
    let jwk_patch_script = r#"
script {
    use aptos_framework::jwks;
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let alice_jwk_0 = jwks::new_unsupported_jwk(b"alice_jwk_id_0", b"alice_jwk_payload_0");
        let patches = vector[
            jwks::new_patch_remove_all(),
            jwks::new_patch_upsert_jwk(b"https://alice.com", alice_jwk_0),
        ];
        jwks::set_patches(&framework_signer, patches);
    }
}
"#;

    let txn_summary = cli.run_script(root_idx, jwk_patch_script).await.unwrap();
    debug!("txn_summary={:?}", txn_summary);

    info!("Use resource API to check the patch result.");
    let patched_jwks = get_patched_jwks(&client).await;
    debug!("patched_jwks={:?}", patched_jwks);

    let expected_providers_jwks = AllProvidersJWKs {
        entries: vec![ProviderJWKs {
            issuer: b"https://alice.com".to_vec(),
            version: 0,
            jwks: vec![JWKMoveStruct::from(JWK::Unsupported(UnsupportedJWK {
                id: b"alice_jwk_id_0".to_vec(),
                payload: b"alice_jwk_payload_0".to_vec(),
            }))],
        }],
    };
    assert_eq!(expected_providers_jwks, patched_jwks.jwks);
}
