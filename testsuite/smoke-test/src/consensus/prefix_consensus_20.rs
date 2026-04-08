// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! 20-validator prefix consensus smoke test.
//!
//! Mirrors the forge `consensus_only_realistic_env_max_tps` setup locally
//! to debug multi-node SPC behavior (View > 1 code paths).

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{
    test_utils::consensus_utils::{no_failure_injection, test_consensus_fault_tolerance},
    Swarm, SwarmExt,
};
use std::sync::Arc;

#[tokio::test]
async fn test_prefix_consensus_20_validators() {
    let num_validators = 20;

    let swarm = SwarmBuilder::new_local(num_validators)
        .with_init_config(Arc::new(move |_, config, _| {
            config.consensus.enable_prefix_consensus = true;
            config.api.failpoints_enabled = true;
            config.consensus.quorum_store_poll_time_ms = 500;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 10;
            config.indexer_db_config.enable_event = true;
        }))
        .with_aptos()
        .build()
        .await;

    let (validator_clients, public_info) = {
        (
            swarm.get_validator_clients_with_names(),
            swarm.aptos_public_info(),
        )
    };

    test_consensus_fault_tolerance(
        validator_clients,
        public_info,
        3,    // cycles
        30.0, // cycle_duration_s (longer for 20 nodes)
        1,    // parts_in_cycle
        no_failure_injection(),
        Box::new(
            move |_, _executed_epochs, _executed_rounds, executed_transactions, _, _| {
                assert!(
                    executed_transactions >= 4,
                    "no progress with prefix consensus, only {} transactions",
                    executed_transactions
                );
                Ok(())
            },
        ),
        false, // no epoch changes — keep stable validator set
        false, // don't defer check errors
    )
    .await
    .unwrap();
}
