// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::smoke_test_environment::SwarmBuilder;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_forge::Swarm;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use std::sync::Arc;

/// Generate deterministic accounts from a fixed seed so we can set the tracing
/// allowlist in node config BEFORE the swarm starts.
fn generate_accounts(num: usize) -> Vec<LocalAccount> {
    use aptos_crypto::Uniform;
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(42);
    (0..num)
        .map(|_| {
            let key = Ed25519PrivateKey::generate(&mut rng);
            let pubkey = aptos_crypto::ed25519::Ed25519PublicKey::from(&key);
            let address = AuthenticationKey::ed25519(&pubkey).account_address();
            LocalAccount::new(address, AccountKey::from_private_key(key), 0)
        })
        .collect()
}

/// Smoke test for transaction tracing:
/// 1. Pre-generates deterministic accounts
/// 2. Configures the swarm with those addresses in the tracing allowlist
/// 3. Creates, funds, and sends transactions from those accounts
/// 4. Verifies TxnTrace log entries appear in validator logs
#[tokio::test]
async fn test_transaction_tracing() {
    let accounts = generate_accounts(5);
    let addresses: Vec<AccountAddress> = accounts.iter().map(|a| a.address()).collect();

    // Configure tracing allowlist in node config before swarm starts
    let addrs = addresses.clone();
    let init_config = Arc::new(
        move |_i: usize,
              config: &mut aptos_config::config::NodeConfig,
              _vfn: &mut aptos_config::config::NodeConfig| {
            config.transaction_tracing.enabled = true;
            config.transaction_tracing.batch_sample_rate = 1.0;
            config.transaction_tracing.txn_sample_rate = 1.0;
            config.transaction_tracing.filter.sender_allowlist = addrs.to_vec();
        },
    );

    let swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(init_config)
        .build()
        .await;

    let mut info = swarm.aptos_public_info();

    // Fund the pre-generated accounts via mint (creates account on-chain + funds)
    let mut accounts = accounts;
    for account in &accounts {
        info.mint(account.address(), 10_000_000_000).await.unwrap();
    }
    println!("Funded {} tracked accounts", accounts.len());

    // Submit transactions from tracked accounts
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

    // Wait for tracing logs to be flushed
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Collect every TxnTrace log line from every validator and parse the
    // structured-JSON payload appended by aptos_logger.
    let mut parsed_traces: Vec<serde_json::Value> = Vec::new();
    for validator in swarm.validators() {
        let logs = validator.get_log_contents().unwrap_or_default();
        let trace_lines: Vec<&str> = logs
            .lines()
            .filter(|l| l.contains("\"event\":\"TxnTrace\""))
            .collect();
        println!(
            "Validator {} has {} TxnTrace log entries",
            validator.peer_id(),
            trace_lines.len()
        );
        for line in &trace_lines {
            println!("  {}", line);
            // aptos_logger appends ` {...json...}` at the end of the line in
            // text mode; locate the trailing JSON object and parse it.
            let json_start = line
                .find("{\"event\"")
                .or_else(|| line.find('{'))
                .expect("TxnTrace log line should contain a JSON payload");
            let json_str = &line[json_start..];
            let value: serde_json::Value = serde_json::from_str(json_str)
                .unwrap_or_else(|e| panic!("failed to parse TxnTrace JSON: {} in {}", e, json_str));
            parsed_traces.push(value);
        }
    }

    assert!(
        !parsed_traces.is_empty(),
        "Expected TxnTrace log entries in validator logs, found none."
    );
    println!("Found {} total TxnTrace log entries", parsed_traces.len());

    // Per-stage delta vectors we expect to be present on a committed trace.
    let required_delta_fields = [
        "mempool_insert_ms",
        "qs_batch_pull_ms",
        "block_proposed_ms",
        "executed_ms",
        "committed_ms",
        "mempool_commit_ms",
    ];

    // Match the bare-hex form produced by serde for AccountAddress.
    let tracked_hex: Vec<String> = addresses.iter().map(|a| a.to_hex()).collect();
    let mut committed_with_tracked_sender = 0;

    for trace in &parsed_traces {
        // Basic identity fields. HashValue and AccountAddress both serialize
        // via serde as bare hex (no `0x` prefix) — their Display impls print
        // `0x...` but the on-wire JSON does not. Humio queries against
        // `data.hash` / `data.sender` therefore use bare hex.
        assert_eq!(trace["event"], "TxnTrace");
        let hash = trace["hash"].as_str().expect("hash must be string");
        assert!(
            !hash.starts_with("0x") && hash.len() == 64,
            "hash should be bare 64-char hex: {}",
            hash
        );
        let sender = trace["sender"].as_str().expect("sender must be string");
        assert!(
            !sender.starts_with("0x") && sender.len() == 64,
            "sender should be bare 64-char hex: {}",
            sender
        );

        let outcome = trace["outcome"].as_str().expect("outcome must be string");
        let attempts = trace["attempts"]
            .as_u64()
            .expect("attempts must be unsigned int") as usize;
        assert!(attempts >= 1, "attempts must be >= 1, got {}", attempts);

        let total_latency_ms = trace["total_latency_ms"]
            .as_i64()
            .expect("total_latency_ms must be int");

        // `stages` is the human-readable timeline string.
        let stages = trace["stages"].as_str().expect("stages must be string");
        assert!(
            stages.contains("MempoolInsert="),
            "stages should contain MempoolInsert= marker: {}",
            stages
        );

        // Per-stage absolute-latency vectors: each must be a Vec<Option<i64>>
        // of length == attempts, and every populated value must be non-
        // negative. Track the max across all populated entries — it must
        // equal `total_latency_ms` for a committed trace.
        let mut max_abs: i64 = 0;
        for (key, val) in trace.as_object().expect("trace must be object").iter() {
            if !key.ends_with("_ms") || key == "total_latency_ms" || key == "age_ms" {
                continue;
            }
            let arr = match val.as_array() {
                Some(a) => a,
                None => continue,
            };
            assert_eq!(
                arr.len(),
                attempts,
                "field {} has length {} but attempts={}",
                key,
                arr.len(),
                attempts
            );
            for entry in arr {
                if entry.is_null() {
                    continue;
                }
                let d = entry
                    .as_i64()
                    .unwrap_or_else(|| panic!("{} entry must be i64 or null, got {}", key, entry));
                assert!(d >= 0, "abs latency {} for {} must be non-negative", d, key);
                if d > max_abs {
                    max_abs = d;
                }
            }
        }

        // Only commit-outcome traces should have the max-equals-total
        // invariant (other outcomes like eviction may be truncated).
        if outcome == "committed" {
            // `total_latency_ms` is (last_stage - mempool_insert) in ms. With
            // absolute-from-base per-stage values, the max populated entry is
            // the last stage chronologically — same as total_latency_ms.
            // Allow ±1ms tolerance for integer division rounding.
            let diff = (max_abs - total_latency_ms).abs();
            assert!(
                diff <= 1,
                "max abs latency {} != total_latency_ms {} (diff={}) for {}",
                max_abs,
                total_latency_ms,
                diff,
                hash
            );
            // For committed traces we should see the terminal pipeline stages.
            for f in &required_delta_fields {
                assert!(
                    trace.get(f).is_some(),
                    "committed trace {} missing per-stage field {}",
                    hash,
                    f
                );
            }
            assert!(
                stages.contains("Committed="),
                "committed trace stages should mention Committed=: {}",
                stages
            );
            if tracked_hex.iter().any(|h| h == sender) {
                committed_with_tracked_sender += 1;
            }
        }
    }

    assert!(
        committed_with_tracked_sender > 0,
        "Expected at least one committed TxnTrace from a tracked sender, found none."
    );
    println!(
        "Verified {} committed TxnTrace entries from tracked senders",
        committed_with_tracked_sender
    );
}
