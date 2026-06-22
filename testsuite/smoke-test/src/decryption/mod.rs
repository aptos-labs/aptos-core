// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod secret_share_rpc_path;

use crate::{
    smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic,
    utils::create_and_fund_account,
};
use aptos_forge::{EmitJobMode, LocalSwarm, NodeExt, Swarm, SwarmExt, TransactionType};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_types::{
    block_metadata_ext::BlockMetadataExt,
    on_chain_config::{FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use std::{sync::Arc, time::Duration};

/// Wait until the ledger reaches the given epoch, returning the encryption key bytes if present.
async fn wait_for_epoch(client: &Client, target_epoch: u64, timeout_secs: u64) -> Option<Vec<u8>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let state = client
            .get_ledger_information()
            .await
            .expect("failed to get ledger info")
            .into_inner();
        if state.epoch >= target_epoch {
            return state.encryption_key;
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "timed out waiting for epoch {}, current epoch is {}",
                target_epoch, state.epoch
            );
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Count the number of encrypted user transactions in the range [start_version, end_version).
async fn count_encrypted_txns(client: &Client, start_version: u64, end_version: u64) -> (u64, u64) {
    let mut count = 0u64;
    let mut decrypted_count = 0u64;
    let page_size = 100u16;
    let mut cursor = start_version;
    while cursor < end_version {
        let limit = std::cmp::min(page_size as u64, end_version - cursor) as u16;
        let txns = client
            .get_transactions_bcs(Some(cursor), Some(limit))
            .await
            .expect("failed to get transactions")
            .into_inner();
        for txn_data in &txns {
            if let Some(signed_txn) = txn_data.transaction.try_as_signed_user_txn() {
                if let Some(payload) = signed_txn.payload().as_encrypted_payload() {
                    count += 1;
                    if !payload.is_encrypted() {
                        decrypted_count += 1;
                    }
                }
            }
        }
        cursor += txns.len() as u64;
        if txns.is_empty() {
            break;
        }
    }
    (count, decrypted_count)
}

/// The `BlockMetadataExt` variant of a committed block-metadata transaction,
/// paired with the epoch it was emitted in. The variant is what each validator
/// chose to emit for that block:
///   - `V1`: decryption disabled.
///   - `V2`: decryption enabled, `PerBlockDecryptionKeyV2` resource absent
///     (legacy mode, digest keyed by consensus round).
///   - `V3`: decryption enabled, resource present (dense decryption round).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BlockMetadataVariant {
    epoch: u64,
    version: u8,
}

/// Scan committed block-metadata transactions in [start_version, end_version)
/// and return the `(epoch, BlockMetadataExt version)` of each. Used to assert
/// the V1/V2/V3 transitions by inspecting exactly what was committed.
async fn scan_block_metadata_variants(
    client: &Client,
    start_version: u64,
    end_version: u64,
) -> Vec<BlockMetadataVariant> {
    let mut variants = Vec::new();
    let page_size = 100u16;
    let mut cursor = start_version;
    while cursor < end_version {
        let limit = std::cmp::min(page_size as u64, end_version - cursor) as u16;
        let txns = client
            .get_transactions_bcs(Some(cursor), Some(limit))
            .await
            .expect("failed to get transactions")
            .into_inner();
        if txns.is_empty() {
            break;
        }
        for txn_data in &txns {
            if let Some(bme) = txn_data.transaction.try_as_block_metadata_ext() {
                let version = match bme {
                    BlockMetadataExt::V0(_) => 0,
                    BlockMetadataExt::V1(_) => 1,
                    BlockMetadataExt::V2(_) => 2,
                    BlockMetadataExt::V3(_) => 3,
                };
                variants.push(BlockMetadataVariant {
                    epoch: bme.epoch(),
                    version,
                });
            }
        }
        cursor += txns.len() as u64;
    }
    variants
}

async fn create_swarm_with_encryption(num_validators: usize) -> LocalSwarm {
    SwarmBuilder::new_local(num_validators)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 10;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
        }))
        .build()
        .await
}

/// Smoke test that verifies:
/// 1. An encryption key exists after epoch 2.
/// 2. The encryption key changes between epochs.
/// 3. Encrypted transactions are committed (via the emitter).
#[tokio::test]
async fn test_encryption_key_rotation_and_encrypted_txns() {
    let num_validators = 4;
    let mut swarm = create_swarm_with_encryption(num_validators).await;

    let client = swarm.validators().last().unwrap().rest_client();

    // ---- Wait for epoch 2 and record the encryption key ----
    info!("Waiting for epoch 2...");
    let key_at_epoch2 = wait_for_epoch(&client, 2, 90).await;
    let epoch2 = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "Reached epoch {} with encryption key present: {}",
        epoch2,
        key_at_epoch2.is_some()
    );
    assert!(
        key_at_epoch2.is_some(),
        "Encryption key should exist after epoch 2, but was None"
    );

    // Record the ledger version so we can scan transactions later.
    let version_before_traffic = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;

    // ---- Use the emitter to generate encrypted traffic ----
    info!("Emitting encrypted traffic...");
    let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let stats = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        200,
        vec![vec![(TransactionType::default(), 1)]],
        true,
        Some(EmitJobMode::MaxLoad {
            mempool_backlog: 20,
        }),
    )
    .await
    .unwrap();
    info!(
        "Emitter stats: submitted={}, committed={}",
        stats.submitted, stats.committed
    );
    assert!(
        stats.committed > 0,
        "Expected some committed transactions from the emitter, got 0"
    );

    // ---- Wait for the next epoch and check the key changed ----
    info!("Waiting for epoch {}...", epoch2 + 1);
    let key_at_next_epoch = wait_for_epoch(&client, epoch2 + 1, 60).await;
    let next_epoch = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "Reached epoch {} with encryption key present: {}",
        next_epoch,
        key_at_next_epoch.is_some()
    );

    assert!(
        key_at_next_epoch.is_some(),
        "Encryption key should exist at epoch {}, but was None",
        next_epoch
    );
    assert_ne!(
        key_at_epoch2.unwrap(),
        key_at_next_epoch.unwrap(),
        "Encryption key must change between epoch {} and epoch {}",
        epoch2,
        next_epoch,
    );

    // ---- Count encrypted transactions in the committed history ----
    let final_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;

    let (encrypted_count, decrypted_count) =
        count_encrypted_txns(&client, version_before_traffic, final_version).await;
    info!(
        "Found {} encrypted transactions ({} decrypted) between version {} and {}",
        encrypted_count, decrypted_count, version_before_traffic, final_version
    );
    assert!(
        encrypted_count > 0,
        "Expected encrypted transactions to be committed, but found 0 in versions [{}, {})",
        version_before_traffic,
        final_version
    );
    assert!(
        decrypted_count > 0,
        "Expected decrypted encrypted transactions to be committed, but found 0 in versions [{}, {})",
        version_before_traffic,
        final_version
    );
}

/// Smoke test that verifies fee-payer encrypted transactions work end-to-end:
/// 1. Builds an encrypted entry-function payload.
/// 2. Signs with the FeePayer authenticator.
/// 3. Submits and verifies the transaction is committed and decrypted.
#[tokio::test]
async fn test_fee_payer_encrypted_transaction() {
    let mut swarm = create_swarm_with_encryption(4).await;

    let client = swarm.validators().last().unwrap().rest_client();

    // Wait for epoch 2 so an encryption key is available.
    info!("Waiting for epoch 2 for encryption key...");
    let key_bytes = wait_for_epoch(&client, 2, 90).await;
    assert!(
        key_bytes.is_some(),
        "Encryption key should exist after epoch 2"
    );
    let key_bytes = key_bytes.unwrap();

    let state = client.get_ledger_information().await.unwrap().into_inner();

    // Build a TransactionFactory with the encryption key and non-zero gas price
    // (GAS_UNIT_PRICE is 0 in test builds).
    let txn_factory = TransactionFactory::new(swarm.chain_id())
        .with_gas_unit_price(200)
        .with_max_gas_amount(10_000);
    txn_factory
        .update_encryption_key_state(state.epoch, Some(&key_bytes))
        .expect("failed to set encryption key");

    // Create and fund sender and fee-payer accounts.
    let sender = create_and_fund_account(&mut swarm, 10_000).await;
    let fee_payer = create_and_fund_account(&mut swarm, 10_000_000).await;

    const APT_COIN: &str = "0x1::aptos_coin::AptosCoin";

    let sender_balance_before = client
        .view_account_balance_bcs_impl(sender.address(), APT_COIN, None)
        .await
        .unwrap()
        .into_inner();
    let fee_payer_balance_before = client
        .view_account_balance_bcs_impl(fee_payer.address(), APT_COIN, None)
        .await
        .unwrap()
        .into_inner();

    // Record version before submission.
    let version_before = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;

    // Re-fetch the encryption key right before building the transaction, in case
    // the epoch has changed while we were funding accounts.
    let state = client.get_ledger_information().await.unwrap().into_inner();
    if let Some(fresh_key) = &state.encryption_key {
        txn_factory
            .update_encryption_key_state(state.epoch, Some(fresh_key))
            .expect("failed to update encryption key");
    }

    // Build and sign an encrypted fee-payer transaction (simple coin transfer to self).
    let payload = aptos_cached_packages::aptos_stdlib::aptos_coin_transfer(sender.address(), 1);
    let builder = txn_factory.payload(payload);
    let signed_txn = sender.sign_fee_payer_with_transaction_builder(vec![], &fee_payer, builder);

    // Verify the built transaction has an encrypted payload.
    assert!(
        signed_txn.payload().is_encrypted_variant(),
        "Transaction payload should be encrypted"
    );

    info!(
        "Submitting fee-payer encrypted txn: sender={}, fee_payer={}",
        sender.address(),
        fee_payer.address()
    );
    let committed_txn = client
        .submit_and_wait(&signed_txn)
        .await
        .expect("fee-payer encrypted transaction should commit")
        .into_inner();

    // Verify the transaction succeeded.
    assert!(
        committed_txn.success(),
        "Fee-payer encrypted transaction should succeed"
    );

    // Verify the encrypted transaction was charged the decryption surcharge.
    let encrypted_gas_used = committed_txn.transaction_info().unwrap().gas_used.0;
    info!("Encrypted transfer gas_used: {}", encrypted_gas_used);
    // The decryption surcharge is 375 external gas units (375_000_000 internal / 1_000_000).
    // The encrypted txn gas should be well above a plain transfer due to this surcharge.
    assert!(
        encrypted_gas_used > 375,
        "Encrypted txn gas_used ({}) should include the decryption surcharge (375)",
        encrypted_gas_used
    );

    // Verify the committed transaction was decrypted.
    let final_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;
    let (encrypted_count, decrypted_count) =
        count_encrypted_txns(&client, version_before, final_version).await;
    info!(
        "Found {} encrypted transactions ({} decrypted) in versions [{}, {})",
        encrypted_count, decrypted_count, version_before, final_version
    );
    assert!(
        decrypted_count > 0,
        "Expected the fee-payer encrypted transaction to be decrypted, found 0"
    );

    // Verify gas was charged to the fee payer, not the sender.
    let sender_balance_after = client
        .view_account_balance_bcs_impl(sender.address(), APT_COIN, None)
        .await
        .unwrap()
        .into_inner();
    let fee_payer_balance_after = client
        .view_account_balance_bcs_impl(fee_payer.address(), APT_COIN, None)
        .await
        .unwrap()
        .into_inner();

    // Sender transferred 1 coin to self (no net change). No gas should be charged to sender.
    assert_eq!(
        sender_balance_before, sender_balance_after,
        "Sender balance should not change (gas charged to fee payer)"
    );
    assert!(
        fee_payer_balance_after < fee_payer_balance_before,
        "Fee payer balance should decrease (gas charged to fee payer): before={}, after={}",
        fee_payer_balance_before,
        fee_payer_balance_after
    );
}

/// Smoke test: the V2 -> V3 block-metadata transition.
///
/// Production genesis creates the `PerBlockDecryptionKeyV2` resource (vm-genesis
/// calls `decryption::initialize`), so a fresh decryption-enabled chain emits
/// `BlockMetadataExt::V3` from epoch 1 and never passes through V2. To exercise
/// the V2->V3 cutover this test sets `initialize_decryption_at_genesis = false`,
/// booting in the pre-resource state: the genesis epoch has decryption on but
/// the resource absent, so it emits the legacy `V2`. The first
/// `reconfiguration_with_dkg` then lazily creates the resource via
/// `decryption::on_new_epoch`, flipping the chain to `V3`. This asserts that
/// exact cutover by inspecting committed block-metadata transactions, and that
/// encrypted txns commit and decrypt under V3.
#[tokio::test]
async fn test_decryption_v2_to_v3_transition() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 10;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
            // Skip creating PerBlockDecryptionKeyV2 at genesis so the chain
            // boots in the legacy (V2) state and flips to V3 at the first
            // reconfiguration_with_dkg. See the doc comment above.
            conf.initialize_decryption_at_genesis = false;
        }))
        .build()
        .await;
    let client = swarm.validators().last().unwrap().rest_client();

    // Advance past the first DKG reconfig so both the genesis-epoch V2 blocks
    // and the post-resource V3 blocks exist in history.
    info!("Waiting for epoch 3 so both V2 (genesis epoch) and V3 (post-resource) blocks exist...");
    let _ = wait_for_epoch(&client, 3, 120).await;

    let final_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;
    let variants = scan_block_metadata_variants(&client, 0, final_version).await;

    let v2_epochs: Vec<u64> = variants
        .iter()
        .filter(|v| v.version == 2)
        .map(|v| v.epoch)
        .collect();
    let v3_epochs: Vec<u64> = variants
        .iter()
        .filter(|v| v.version == 3)
        .map(|v| v.epoch)
        .collect();
    info!("V2 epochs: {:?}, V3 epochs: {:?}", v2_epochs, v3_epochs);

    assert!(
        !v2_epochs.is_empty(),
        "expected V2 block metadata before PerBlockDecryptionKeyV2 was created"
    );
    assert!(
        !v3_epochs.is_empty(),
        "expected V3 block metadata after PerBlockDecryptionKeyV2 was created"
    );
    assert!(
        v2_epochs.iter().max().unwrap() < v3_epochs.iter().min().unwrap(),
        "V2 -> V3 must be a clean monotonic cutover: all V2 epochs precede all V3 epochs (V2: {:?}, V3: {:?})",
        v2_epochs,
        v3_epochs,
    );
    // Decryption is on for the whole run, so V1 must never appear.
    assert!(
        !variants.iter().any(|v| v.version == 1),
        "did not expect V1 metadata when decryption is enabled at genesis (got {:?})",
        variants,
    );

    // Emit encrypted traffic under V3 and verify it commits + decrypts.
    let version_before_traffic = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;
    let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let stats = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        200,
        vec![vec![(TransactionType::default(), 1)]],
        true,
        Some(EmitJobMode::MaxLoad {
            mempool_backlog: 20,
        }),
    )
    .await
    .unwrap();
    assert!(
        stats.committed > 0,
        "expected some committed transactions from the emitter, got 0"
    );

    let final_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;
    let (encrypted_count, decrypted_count) =
        count_encrypted_txns(&client, version_before_traffic, final_version).await;
    info!(
        "Found {} encrypted txns ({} decrypted) under V3",
        encrypted_count, decrypted_count
    );
    assert!(
        decrypted_count > 0,
        "expected encrypted txns to be decrypted under V3"
    );
    // Everything emitted during the V3 traffic window must be V3.
    let traffic_variants =
        scan_block_metadata_variants(&client, version_before_traffic, final_version).await;
    assert!(
        traffic_variants.iter().all(|v| v.version == 3),
        "all block metadata during the V3 traffic window should be V3, got {:?}",
        traffic_variants,
    );
}

/// Smoke test: the V1 -> V3 block-metadata transition.
///
/// Chunky DKG (and thus decryption) starts disabled, so the chain emits
/// `BlockMetadataExt::V1`. Randomness DKG is on from genesis, so the
/// `PerBlockDecryptionKeyV2` resource is created by the first
/// `reconfiguration_with_dkg` well before decryption is ever turned on. When
/// chunky DKG + `ENCRYPTED_TRANSACTIONS` are enabled via governance, the
/// resource already exists, so the chain jumps straight from V1 to V3 -- it
/// never passes through V2. This asserts that path and that encrypted txns work
/// afterwards.
#[tokio::test]
async fn test_decryption_v1_to_v3_transition() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 10;
            conf.consensus_config.enable_validator_txns();
            // Randomness on (so reconfiguration_with_dkg runs and creates the
            // PerBlockDecryptionKeyV2 resource early), but chunky DKG and the
            // ENCRYPTED_TRANSACTIONS feature off -> decryption disabled -> V1.
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_disabled());
            let mut features = Features::default();
            features.disable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);
    let client = swarm.validators().last().unwrap().rest_client();

    // Advance a few epochs with decryption disabled and confirm V1 only.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(60))
        .await
        .expect("waited too long for epoch 3");
    let version_before_enable = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;
    let pre = scan_block_metadata_variants(&client, 0, version_before_enable).await;
    info!("Pre-enable block metadata variants: {:?}", pre);
    assert!(
        pre.iter().any(|v| v.version == 1),
        "expected V1 block metadata before decryption is enabled"
    );
    assert!(
        !pre.iter().any(|v| v.version == 2 || v.version == 3),
        "should not see V2/V3 before decryption is enabled (got {:?})",
        pre,
    );

    // Enable chunky DKG config + ENCRYPTED_TRANSACTIONS (feature flag 108) via governance.
    info!("Enabling chunky DKG config and ENCRYPTED_TRANSACTIONS at runtime.");
    let script = r#"
script {
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::features;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let config = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3)
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, config);

        features::change_feature_flags_for_next_epoch(&framework_signer, vector[108], vector[]);

        aptos_governance::reconfigure(&framework_signer);
    }
}
"#;
    cli.run_script(root_idx, script)
        .await
        .expect("governance script execution failed");

    // Wait for the encryption key to appear (chunky DKG completed -> decryption live).
    info!("Waiting for encryption key after enabling decryption...");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let version_after_enable = loop {
        let state = client.get_ledger_information().await.unwrap().into_inner();
        if state.encryption_key.is_some() {
            info!(
                "Encryption key present at epoch {} version {}",
                state.epoch, state.version
            );
            break state.version;
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "timed out waiting for encryption key after enabling decryption (epoch {})",
                state.epoch
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    // Emit encrypted traffic and confirm decryption works under V3.
    let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let stats = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        200,
        vec![vec![(TransactionType::default(), 1)]],
        true,
        Some(EmitJobMode::MaxLoad {
            mempool_backlog: 20,
        }),
    )
    .await
    .unwrap();
    assert!(
        stats.committed > 0,
        "expected some committed transactions from the emitter, got 0"
    );

    let final_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;

    // Scan the entire history: V1 then V3, and never V2 (the resource already
    // existed when decryption turned on, so the cutover skips legacy mode).
    let all_variants = scan_block_metadata_variants(&client, 0, final_version).await;
    let v1_epochs: Vec<u64> = all_variants
        .iter()
        .filter(|v| v.version == 1)
        .map(|v| v.epoch)
        .collect();
    let v3_epochs: Vec<u64> = all_variants
        .iter()
        .filter(|v| v.version == 3)
        .map(|v| v.epoch)
        .collect();
    info!("V1 epochs: {:?}, V3 epochs: {:?}", v1_epochs, v3_epochs);
    assert!(
        !v1_epochs.is_empty(),
        "expected V1 block metadata before decryption is enabled"
    );
    assert!(
        !v3_epochs.is_empty(),
        "expected V3 block metadata after decryption is enabled"
    );
    assert!(
        !all_variants.iter().any(|v| v.version == 2),
        "V1 -> V3 must skip V2: the resource already existed when decryption turned on (got {:?})",
        all_variants,
    );
    assert!(
        v1_epochs.iter().max().unwrap() < v3_epochs.iter().min().unwrap(),
        "V1 -> V3 must be a clean monotonic cutover (V1: {:?}, V3: {:?})",
        v1_epochs,
        v3_epochs,
    );

    let (encrypted_count, decrypted_count) =
        count_encrypted_txns(&client, version_after_enable, final_version).await;
    info!(
        "Found {} encrypted txns ({} decrypted) after v1->v3",
        encrypted_count, decrypted_count
    );
    assert!(
        decrypted_count > 0,
        "expected encrypted txns to be decrypted under V3"
    );
}
