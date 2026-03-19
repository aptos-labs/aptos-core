// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use aptos_cached_packages::aptos_stdlib;
use aptos_forge::{Node, Swarm};
use reqwest::Url;

/// Smoke test for transaction tracing:
/// 1. Spins up a local swarm with 4 validators
/// 2. Creates and funds 5 accounts
/// 3. POSTs a tracing filter with those 5 sender addresses to each validator
/// 4. Submits coin transfer transactions from the tracked accounts
/// 5. Verifies the transactions commit (tracing doesn't break the pipeline)
/// 6. GETs the tracing filter back and verifies it matches
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

    // Step 2: POST the tracing filter to each validator's inspection service
    let client = reqwest::Client::new();
    let filter_json = serde_json::json!({
        "enabled": true,
        "sender_allowlist": tracked_addresses,
    });

    let validator_endpoints: Vec<(_, Url)> = swarm
        .validators()
        .map(|v| {
            let mut url = v.inspection_service_endpoint();
            url.set_path("transaction_tracing");
            (v.peer_id(), url)
        })
        .collect();

    for (peer_id, url) in &validator_endpoints {
        let resp = client
            .post(url.clone())
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

    // Step 3: Verify the filter was set via GET
    let (_, url) = &validator_endpoints[0];
    let resp = client.get(url.clone()).send().await.unwrap();
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap();
    println!("GET filter: status={}, body={}", status, body);
    assert!(status.is_success());
    assert_eq!(body["enabled"], true);
    let allowlist = body["sender_allowlist"].as_array().unwrap();
    assert_eq!(allowlist.len(), num_accounts);

    // Step 4: Submit transactions from tracked accounts and verify they commit
    // This proves that tracing doesn't interfere with normal transaction processing
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

    // We should see at least some TxnTrace entries across all validators.
    // The leader validator that proposes the block with our transactions will have traces.
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

    // Step 7: Disable tracing and verify
    let disable_json = serde_json::json!({
        "enabled": false,
        "sender_allowlist": [],
    });
    let (_, url) = &validator_endpoints[0];
    let resp = client
        .post(url.clone())
        .json(&disable_json)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    let resp = client.get(url.clone()).send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["enabled"], false);
    println!("Tracing disabled successfully");
}
