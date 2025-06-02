// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::txn_emitter::generate_traffic;
use aptos_forge::{NodeExt, Swarm, TransactionType};
use aptos_types::PeerId;
use std::time::Duration;

pub async fn generate_traffic_and_assert_committed(
    swarm: &mut dyn Swarm,
    nodes: &[PeerId],
    duration: Duration,
) {
    let rest_client = swarm.validator(nodes[0]).unwrap().rest_client();

    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    let txn_stat = generate_traffic(swarm, nodes, duration, 100, vec![vec![
        (
            TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool: false,
                non_conflicting: false,
                use_fa_transfer: true,
            },
            70,
        ),
        (
            TransactionType::AccountGeneration {
                add_created_accounts_to_pool: true,
                max_account_working_set: 1_000_000,
                creation_balance: 5_000_000,
            },
            20,
        ),
    ]])
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate());
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 30);
    assert!(txn_stat.committed > 30);
}
