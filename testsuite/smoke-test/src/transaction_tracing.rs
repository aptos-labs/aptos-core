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

    // Check validator logs for TxnTrace entries
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
        "Expected TxnTrace log entries in validator logs, found none."
    );
    println!(
        "Found {} total TxnTrace log entries across all validators",
        total_trace_count
    );

    // Verify at least one tracked address appears in the trace logs
    let all_logs: String = swarm
        .validators()
        .filter_map(|v| v.get_log_contents().ok())
        .collect::<Vec<_>>()
        .join("\n");

    let tracked_hex: Vec<String> = addresses.iter().map(|a| a.to_hex()).collect();
    let mut found_addresses = 0;
    for addr in &tracked_hex {
        if all_logs.contains(addr) {
            found_addresses += 1;
            println!("Found traced address {} in validator logs", addr);
        }
    }
    assert!(
        found_addresses > 0,
        "Expected at least one tracked sender address in TxnTrace logs, found none"
    );
}
