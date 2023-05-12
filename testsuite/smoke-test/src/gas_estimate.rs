// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic};
use aptos_forge::{NodeExt, TransactionType};
use aptos_global_constants::{DEFAULT_BUCKETS, GAS_UNIT_PRICE};
use std::{sync::Arc, time::Duration};

fn next_bucket(gas_unit_price: u64) -> u64 {
    *DEFAULT_BUCKETS
        .iter()
        .find(|bucket| **bucket > gas_unit_price)
        .unwrap()
}

#[tokio::test]
async fn test_gas_estimate() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.gas_estimate_full_block_threshold = 1;
            // TODO: configure consensus to have faster and consecutive non-empty rounds
        }))
        .build()
        .await;
    let client = swarm.validators().next().unwrap().rest_client();
    let estimation = match client.estimate_gas_price().await {
        Ok(res) => res.into_inner(),
        Err(e) => panic!("Client error: {:?}", e),
    };
    println!("{:?}", estimation);
    // Note: in testing GAS_UNIT_PRICE = 0
    assert_eq!(Some(GAS_UNIT_PRICE), estimation.deprioritized_gas_estimate);
    assert_eq!(GAS_UNIT_PRICE, estimation.gas_estimate);
    assert_eq!(
        Some(next_bucket(GAS_UNIT_PRICE)),
        estimation.prioritized_gas_estimate
    );

    let txn_gas_price = 100;
    let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let txn_stat = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(40),
        txn_gas_price,
        vec![vec![(
            TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool: false,
            },
            70,
        )]],
    )
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate());

    let estimation = match client.estimate_gas_price().await {
        Ok(res) => res.into_inner(),
        Err(e) => panic!("Client error: {:?}", e),
    };
    println!("{:?}", estimation);
    // Note: it's quite hard to get deprioritized_gas_estimate higher in smoke tests
    assert_eq!(txn_gas_price + 1, estimation.gas_estimate);
    assert_eq!(
        Some(next_bucket(txn_gas_price)),
        estimation.prioritized_gas_estimate
    );

    // Empty blocks will reset the prices
    std::thread::sleep(Duration::from_secs(40));
    let estimation = match client.estimate_gas_price().await {
        Ok(res) => res.into_inner(),
        Err(e) => panic!("Client error: {:?}", e),
    };
    println!("{:?}", estimation);
    // Note: in testing GAS_UNIT_PRICE = 0
    assert_eq!(Some(GAS_UNIT_PRICE), estimation.deprioritized_gas_estimate);
    assert_eq!(GAS_UNIT_PRICE, estimation.gas_estimate);
    assert_eq!(
        Some(next_bucket(GAS_UNIT_PRICE)),
        estimation.prioritized_gas_estimate
    );

    panic!("BCHO log")
}
