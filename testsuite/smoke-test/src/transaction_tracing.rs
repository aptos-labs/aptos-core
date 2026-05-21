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

    // Per-stage scalar fields expected on a committed trace's first pipeline
    // pass. Each is `data.<field>` as a JSON number — no array indexing.
    let required_scalar_fields = [
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
            .expect("attempts must be unsigned int");
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

        // Every populated `*_ms` per-stage field must be a plain JSON number
        // (not an array — the schema emits scalars for the first pipeline
        // pass only). Most stages use the local clock and are non-negative;
        // `block_proposed_ms` can be negative due to cross-validator clock
        // skew. We also assert no value exceeds `total_latency_ms` (the
        // structured scalars only cover attempt 1; for a retried trace,
        // total_latency reflects later attempts and stays >= scalar max).
        for (key, val) in trace.as_object().expect("trace must be object").iter() {
            if !key.ends_with("_ms") || key == "total_latency_ms" || key == "age_ms" {
                continue;
            }
            let d = match val.as_i64() {
                Some(n) => n,
                None => panic!("{} must be i64, got {}", key, val),
            };
            // Most stages use local clocks and are non-negative;
            // `block_proposed_ms` and `parent_block_proposed_ms` record
            // foreign-block timestamps (proposer's clock for the child and
            // parent blocks) and can be negative due to cross-validator
            // clock skew.
            if key != "block_proposed_ms" && key != "parent_block_proposed_ms" {
                assert!(d >= 0, "abs latency {} for {} must be non-negative", d, key);
            }
            assert!(
                d <= total_latency_ms,
                "{} = {} exceeds total_latency_ms = {} (hash={})",
                key,
                d,
                total_latency_ms,
                hash
            );
        }

        if outcome == "committed" {
            // First-attempt scalars for the terminal pipeline stages must be
            // present. For a single-attempt commit, `mempool_commit_ms` equals
            // `total_latency_ms`; for a retried-then-committed trace, the
            // structured fields only cover attempt 1 (no MempoolCommit there),
            // so this required check is gated to single-attempt traces.
            if attempts == 1 {
                for f in &required_scalar_fields {
                    assert!(
                        trace.get(f).is_some(),
                        "committed single-attempt trace {} missing scalar field {}",
                        hash,
                        f
                    );
                }
                let mempool_commit_ms = trace["mempool_commit_ms"]
                    .as_i64()
                    .expect("mempool_commit_ms must be int on single-attempt commit");
                let diff = (mempool_commit_ms - total_latency_ms).abs();
                assert!(
                    diff <= 1,
                    "mempool_commit_ms {} != total_latency_ms {} (diff={}) for {}",
                    mempool_commit_ms,
                    total_latency_ms,
                    diff,
                    hash
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
