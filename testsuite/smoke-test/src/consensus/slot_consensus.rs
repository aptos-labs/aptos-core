// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{
    test_utils::consensus_utils::{no_failure_injection, test_consensus_fault_tolerance},
    Swarm, SwarmExt,
};
use std::sync::Arc;

#[tokio::test]
async fn test_prefix_consensus_no_failures() {
    let num_validators = 4;

    let swarm = SwarmBuilder::new_local(num_validators)
        .with_init_config(Arc::new(move |_, config, _| {
            config.consensus.enable_prefix_consensus = true;
            config.api.failpoints_enabled = true;
            config.consensus.round_initial_timeout_ms = 1000;
            config.consensus.round_timeout_backoff_exponent_base = 1.0;
            config.consensus.quorum_store_poll_time_ms = 500;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
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
        3,
        8.0,
        1,
        no_failure_injection(),
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                assert!(
                    executed_transactions >= 4,
                    "no progress with active consensus, only {} transactions",
                    executed_transactions
                );
                assert!(
                    executed_epochs >= 1 || executed_rounds >= 2,
                    "no progress with active consensus, only {} epochs, {} rounds",
                    executed_epochs,
                    executed_rounds
                );
                Ok(())
            },
        ),
        true,
        false,
    )
    .await
    .unwrap();
}
