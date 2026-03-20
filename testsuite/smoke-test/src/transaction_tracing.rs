// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use aptos_cached_packages::aptos_stdlib;
use aptos_forge::{
    args::TransactionTypeArg, EmitJobMode, EmitJobRequest, Node, NodeExt, Swarm, TxnEmitter,
};
use aptos_rest_client::aptos_api_types::Transaction;
use aptos_sdk::transaction_builder::TransactionFactory;
use rand::{rngs::OsRng, SeedableRng};
use reqwest::Url;
use std::{collections::HashSet, time::Duration};

/// Build the admin service URL for a validator node.
fn admin_service_url(v: &dyn Node, path: &str) -> Url {
    let port = v.config().admin_service.port;
    let mut url: Url = format!("http://localhost:{}", port).parse().unwrap();
    url.set_path(path);
    url
}

/// Smoke test for transaction tracing:
/// 1. Spins up a local swarm with 4 validators
/// 2. Creates and funds 5 accounts
/// 3. POSTs a tracing filter via the admin service
/// 4. Submits coin transfer transactions from the tracked accounts
/// 5. Verifies the transactions commit (tracing doesn't break the pipeline)
/// 6. GETs the tracing filter back via the inspection service and verifies it
/// 7. Checks validator logs for TxnTrace entries with the tracked sender addresses
#[tokio::test]
async fn test_transaction_tracing() {
    let mut swarm = new_local_swarm_with_aptos(4).await;
    let mut info = swarm.aptos_public_info();

    // Step 1: Create and fund 5 accounts
    let num_accounts = 5;
    let mut accounts = Vec::new();
    for _ in 0..num_accounts {
        let account = info
            .create_and_fund_user_account(10_000_000_000)
            .await
            .unwrap();
        accounts.push(account);
    }

    let tracked_addresses: Vec<String> = accounts.iter().map(|a| a.address().to_hex()).collect();
    println!("Created {} tracked accounts", tracked_addresses.len());

    // Step 2: POST the tracing filter to each validator's admin service
    let client = reqwest::Client::new();
    let filter_json = serde_json::json!({
        "enabled": true,
        "sender_allowlist": tracked_addresses,
    });

    // Collect both admin (POST) and inspection (GET) endpoints per validator
    let endpoints: Vec<(_, Url, Url)> = swarm
        .validators()
        .map(|v| {
            let admin_url = admin_service_url(v, "transaction_tracing");
            let mut inspect_url = v.inspection_service_endpoint();
            inspect_url.set_path("transaction_tracing");
            (v.peer_id(), admin_url, inspect_url)
        })
        .collect();

    for (peer_id, admin_url, _) in &endpoints {
        let resp = client
            .post(admin_url.clone())
            .json(&filter_json)
            .send()
            .await
            .unwrap();
        let status = resp.status();
        let body = resp.text().await.unwrap();
        println!(
            "POST to validator {}: status={}, body={}",
            peer_id, status, body
        );
        assert!(
            status.is_success(),
            "Failed to set tracing filter on validator {}: {} {}",
            peer_id,
            status,
            body
        );
    }

    // Step 3: Verify the filter was set via GET on the inspection service
    let (_, _, inspect_url) = &endpoints[0];
    let resp = client.get(inspect_url.clone()).send().await.unwrap();
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap();
    println!("GET filter: status={}, body={}", status, body);
    assert!(status.is_success());
    assert_eq!(body["enabled"], true);
    let allowlist = body["sender_allowlist"].as_array().unwrap();
    assert_eq!(allowlist.len(), num_accounts);

    // Step 4: Submit transactions from tracked accounts and verify they commit
    let receiver = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();

    for account in &mut accounts {
        let tx = account.sign_with_transaction_builder(
            info.transaction_factory()
                .payload(aptos_stdlib::aptos_coin_transfer(receiver.address(), 100)),
        );
        let pending = info.client().submit(&tx).await.unwrap().into_inner();
        let result = info.client().wait_for_transaction(&pending).await;
        assert!(
            result.is_ok(),
            "Transaction from tracked account {} failed: {:?}",
            account.address(),
            result.err()
        );
        println!(
            "Transaction committed successfully (hash: {})",
            pending.hash
        );
    }

    // Step 5: Wait a moment for tracing logs to be flushed, then check validator logs
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let mut total_trace_count = 0;
    for validator in swarm.validators() {
        let logs = validator.get_log_contents().unwrap_or_default();
        let trace_lines: Vec<&str> = logs.lines().filter(|l| l.contains("TxnTrace")).collect();
        println!(
            "Validator {} has {} TxnTrace log entries",
            validator.peer_id(),
            trace_lines.len()
        );
        for line in &trace_lines {
            println!("  {}", line);
        }
        total_trace_count += trace_lines.len();
    }

    assert!(
        total_trace_count > 0,
        "Expected TxnTrace log entries in validator logs, found none. \
         Tracing instrumentation may not be working."
    );
    println!(
        "Found {} total TxnTrace log entries across all validators",
        total_trace_count
    );

    // Step 6: Verify at least one tracked address appears in the trace logs
    let all_logs: String = swarm
        .validators()
        .filter_map(|v| v.get_log_contents().ok())
        .collect::<Vec<_>>()
        .join("\n");

    let mut found_addresses = 0;
    for addr in &tracked_addresses {
        if all_logs.contains(addr) {
            found_addresses += 1;
            println!("Found traced address {} in validator logs", addr);
        }
    }
    assert!(
        found_addresses > 0,
        "Expected at least one tracked sender address in TxnTrace logs, found none"
    );

    // Step 7: Disable tracing via admin service and verify via inspection service
    let disable_json = serde_json::json!({
        "enabled": false,
        "sender_allowlist": [],
    });
    let (_, admin_url, inspect_url) = &endpoints[0];
    let resp = client
        .post(admin_url.clone())
        .json(&disable_json)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    let resp = client.get(inspect_url.clone()).send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["enabled"], false);
    println!("Tracing disabled successfully");
}

/// Sustained traffic test: discovers emitter sender addresses from on-chain
/// transactions, then enables tracing for those senders while traffic continues.
///
/// Run with:
///   cargo test -p smoke-test -- test_transaction_tracing_wait_summary --ignored --nocapture
#[ignore]
#[tokio::test]
async fn test_transaction_tracing_wait_summary() {
    let swarm = new_local_swarm_with_aptos(4).await;

    let gas_price = 100;
    let validator_clients: Vec<_> = swarm.validators().map(|v| v.rest_client()).collect();

    let emit_job_request = EmitJobRequest::default()
        .rest_clients(validator_clients)
        .gas_price(gas_price)
        .expected_gas_per_txn(10_000_000)
        .max_gas_per_txn(20_000_000)
        .transaction_type(TransactionTypeArg::CoinTransfer.materialize_default())
        .mode(EmitJobMode::ConstTps { tps: 30 });

    let chain_info = swarm.chain_info();
    let txn_factory =
        TransactionFactory::new(chain_info.chain_id).with_gas_unit_price(gas_price);
    let rng = SeedableRng::from_rng(OsRng).unwrap();
    let rest_cli = chain_info.rest_client();
    let mut emitter = TxnEmitter::new(txn_factory, rng, rest_cli.clone());

    // Start traffic in the background
    let phases = emit_job_request.get_num_phases();
    let mut job = emitter
        .start_job(chain_info.root_account, emit_job_request, phases)
        .await
        .unwrap();
    println!("=== Emitter started, traffic flowing ===");

    // Wait for some transactions to land on chain
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Discover sender addresses from recent committed transactions
    let txns = rest_cli
        .get_transactions(None, Some(100))
        .await
        .unwrap()
        .into_inner();

    let mut unique_senders = HashSet::new();
    for txn in &txns {
        if let Transaction::UserTransaction(user_txn) = txn {
            unique_senders.insert(user_txn.request.sender.inner().to_hex());
        }
    }
    let tracked_accounts: Vec<String> = unique_senders.into_iter().take(20).collect();
    println!(
        "Discovered {} sender addresses to trace",
        tracked_accounts.len()
    );

    // Enable tracing for discovered senders on all validators
    let client = reqwest::Client::new();
    let filter_json = serde_json::json!({
        "enabled": true,
        "sender_allowlist": tracked_accounts,
    });

    for validator in swarm.validators() {
        let url = admin_service_url(validator, "transaction_tracing");
        let resp = client
            .post(url)
            .json(&filter_json)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "Failed to set tracing filter on {}",
            validator.peer_id()
        );
    }
    println!("Tracing enabled for {} senders", tracked_accounts.len());

    // Let traffic continue with tracing active
    println!("\n=== Letting traffic run for 20s with tracing ===");
    tokio::time::sleep(Duration::from_secs(20)).await;

    // Stop the emitter
    let stats = job.stop_job().await;
    let stats = &stats[0];
    println!(
        "Traffic stats: submitted={} committed={}",
        stats.submitted, stats.committed,
    );

    // Wait for traces to be flushed
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Collect and print all TxnTrace lines
    println!("\n=== TxnTrace log entries ===");
    let mut total_traces = 0;
    let mut traces_with_wait = 0;
    let mut all_trace_lines: Vec<String> = Vec::new();

    for validator in swarm.validators() {
        let logs = validator.get_log_contents().unwrap_or_default();
        let trace_lines: Vec<String> = logs
            .lines()
            .filter(|l| l.contains("TxnTrace"))
            .map(|l| l.to_string())
            .collect();

        if !trace_lines.is_empty() {
            println!(
                "\n--- Validator {} ({} traces) ---",
                validator.peer_id(),
                trace_lines.len()
            );
            for line in &trace_lines {
                println!("{}", line);
                if line.contains("wait(") {
                    traces_with_wait += 1;
                }
            }
            total_traces += trace_lines.len();
            all_trace_lines.extend(trace_lines);
        }
    }

    println!("\n=== Summary ===");
    println!("Total TxnTrace entries: {}", total_traces);
    println!("Traces with wait() summary: {}", traces_with_wait);

    assert!(
        total_traces > 0,
        "Expected TxnTrace entries but found none"
    );

    // Print a few example wait() summaries for easy inspection
    let wait_lines: Vec<&String> = all_trace_lines
        .iter()
        .filter(|l| l.contains("wait("))
        .collect();
    if !wait_lines.is_empty() {
        println!("\n=== Example wait() summaries (first 10) ===");
        for line in wait_lines.iter().take(10) {
            if let Some(start) = line.find("wait(") {
                if let Some(end) = line[start..].find(") ") {
                    println!("  {}", &line[start..start + end + 1]);
                }
            }
        }
    }
}
