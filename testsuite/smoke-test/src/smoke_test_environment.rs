// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_genesis::builder::InitConfigFn;
use forge::{Factory, LocalFactory, LocalSwarm};
use once_cell::sync::Lazy;
use rand::rngs::OsRng;
use std::num::NonZeroUsize;

pub async fn new_local_swarm(
    num_validators: usize,
    genesis_modules: Option<Vec<Vec<u8>>>,
    init_config: Option<InitConfigFn>,
) -> LocalSwarm {
    static FACTORY: Lazy<LocalFactory> = Lazy::new(|| LocalFactory::from_workspace().unwrap());

    ::aptos_logger::Logger::new().init();
    let version = FACTORY.versions().max().unwrap();

    FACTORY
        .new_swarm_with_version(
            OsRng,
            NonZeroUsize::new(num_validators).unwrap(),
            &version,
            genesis_modules,
            // TODO: migrate to > 0
            0,
            init_config,
        )
        .await
        .unwrap()
}

// Gas is not enabled with this setup, it's enabled via forge instance.
pub async fn new_local_swarm_with_aptos(num_validators: usize) -> LocalSwarm {
    new_local_swarm(
        num_validators,
        Some(cached_framework_packages::module_blobs().to_vec()),
        None,
    )
    .await
}

// Gas is not enabled with this setup, it's enabled via forge instance.
pub async fn new_local_swarm_with_aptos_and_config(
    num_validators: usize,
    init_config: InitConfigFn,
) -> LocalSwarm {
    new_local_swarm(
        num_validators,
        Some(cached_framework_packages::module_blobs().to_vec()),
        Some(init_config),
    )
    .await
}

#[tokio::test]
async fn test_prevent_starting_nodes_twice() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    assert!(swarm.launch().await.is_err());
    let validator = swarm.validators_mut().next().unwrap();
    assert!(validator.start().is_err());
    validator.stop();
    assert!(validator.start().is_ok());
    assert!(validator.start().is_err());
}
