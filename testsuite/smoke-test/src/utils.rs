// Copyright © Aptos Foundation

use aptos_rest_client::Client;
use aptos_types::on_chain_config::OnChainConsensusConfig;
use move_core_types::language_storage::CORE_CODE_ADDRESS;

pub(crate) async fn get_current_version(rest_client: &Client) -> u64 {
    rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .version
}

pub(crate) async fn get_current_consensus_config(rest_client: &Client) -> OnChainConsensusConfig {
    bcs::from_bytes(
        &rest_client
            .get_account_resource_bcs::<Vec<u8>>(
                CORE_CODE_ADDRESS,
                "0x1::consensus_config::ConsensusConfig",
            )
            .await
            .unwrap()
            .into_inner(),
    )
    .unwrap()
}
