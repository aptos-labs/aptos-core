// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err, Result};
use aptos_sdk::transaction_builder::TransactionFactory;
use futures::future::join_all;
use itertools::zip;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_core::OsRng;
use std::{
    cmp::min,
    time::{Duration, Instant},
};
use transaction_emitter_lib::{query_sequence_number, Cluster, TxnEmitter};

pub async fn diag(cluster: &Cluster) -> Result<()> {
    let client = cluster.random_instance().rest_client();
    let mut faucet_account = cluster.load_aptos_root_account(&client).await?;
    let emitter = TxnEmitter::new(
        TransactionFactory::new(cluster.chain_id)
            .with_gas_unit_price(aptos_global_constants::GAS_UNIT_PRICE),
        StdRng::from_seed(OsRng.gen()),
    );
    let faucet_account_address = faucet_account.address();
    let instances: Vec<_> = cluster.all_instances().collect();
    for instance in &instances {
        print!("Submitting txn through {}...", instance);
        let deadline = emitter
            .submit_single_transaction(
                &instance.rest_client(),
                &mut faucet_account,
                &faucet_account_address,
                10,
            )
            .await
            .map_err(|e| format_err!("Failed to submit txn through {}: {:?}", instance, e))?;
        println!("seq={}", faucet_account.sequence_number());
        println!(
            "Waiting all full nodes to get to seq {}",
            faucet_account.sequence_number()
        );
        loop {
            let clients = instances
                .iter()
                .map(|instance| instance.rest_client())
                .collect::<Vec<_>>();
            let futures = clients
                .iter()
                .map(|client| query_sequence_number(client, faucet_account_address));
            let results = join_all(futures).await;
            let mut all_good = true;
            for (instance, result) in zip(instances.iter(), results) {
                let seq = result.map_err(|e| {
                    format_err!("Failed to query sequence number from {}: {:?}", instance, e)
                })?;
                let host = instance.api_url().host().unwrap().to_string();
                let status = if seq != faucet_account.sequence_number() {
                    all_good = false;
                    "good"
                } else {
                    "bad"
                };
                print!("[{}:{}:{}]  ", &host[..min(host.len(), 10)], seq, status);
            }
            println!();
            if all_good {
                break;
            }
            if Instant::now() > deadline {
                bail!("Not all full nodes were updated and transaction expired");
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    println!("Looks like all full nodes are healthy!");
    Ok(())
}
