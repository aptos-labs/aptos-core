// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Watchdog matrix smoke tests (per @zjma's review of #19813).
//!
//! For every reachable combination of DKG state in epoch x-1 (prior) and DKG
//! state in epoch x (current), epoch x+1 must be reachable, with randomness
//! and the per-epoch encryption key behaving as the on-chain DKG outcomes
//! imply. Axes:
//!
//!   prior   randomness DKG ∈ {NotTriggered, Aborted, Finished}
//!   prior   chunky DKG     ∈ {NotTriggered, Aborted, Finished}
//!   current randomness DKG ∈ {Aborted, Finished}
//!   current chunky DKG     ∈ {Aborted, Finished}
//!
//! Two (prior) cells are impossible by framework construction:
//! `reconfiguration_with_dkg::try_start_with_chunky_dkg` always starts BOTH
//! the randomness DKG and the chunky DKG, so the chunky DKG cannot be
//! triggered without the randomness DKG. That eliminates (NotTriggered,
//! Aborted) and (NotTriggered, Finished) from the prior axis, leaving
//! 7 valid prior states × 4 current states = 28 cases.
//!
//! Setup model (no abbreviations, no class A/B/C):
//!
//! 1. Genesis configures the chain so that during epoch x-1 the on-chain
//!    features match the prior axis values:
//!      - prior randomness DKG = NotTriggered  → randomness off, validator
//!        transactions off (so no DKG can run at any boundary).
//!      - prior randomness DKG = Aborted/Finished → randomness on, validator
//!        transactions on, so the gov-triggered x-1 → x reconfig will
//!        actually run a randomness DKG.
//!      - prior chunky DKG = NotTriggered → chunky off, ENCRYPTED_TRANSACTIONS
//!        off.
//!      - prior chunky DKG = Aborted/Finished → chunky on (V1), ENCRYPTED_
//!        TRANSACTIONS on.
//!    The epoch-timeout watchdog is *armed from genesis* via the new
//!    `epoch_timeout_grace_period_secs_override` field in vm-genesis, so the
//!    watchdog is active in every epoch including epoch 1 → 2.
//!
//! 2. Test waits for epoch 2 (= epoch x-1) and publishes the on_chain_dice
//!    module while the chain is stable.
//!
//! 3. Test arms failpoints for the prior outcomes (chunky/randomness DKG
//!    abort failpoints, set only when the prior outcome is Aborted).
//!
//! 4. Test runs a single governance script that buffers whatever has to
//!    flip from off to on for epoch x (validator transactions, randomness,
//!    chunky V1, ENCRYPTED_TRANSACTIONS) and calls
//!    `aptos_governance::reconfigure`. `reconfigure` looks at the *current*
//!    config and decides automatically: if validator transactions or
//!    randomness is off, it goes straight through `finish()` (sync, no DKG);
//!    if randomness is on and chunky is off, it goes async with the
//!    randomness DKG only; if both on, it goes async with both DKGs. That
//!    gov-triggered reconfig is the x-1 → x boundary.
//!
//! 5. After the chain reaches epoch x, the test adjusts the failpoints to
//!    match the current outcomes and waits for the natural end-of-epoch-x
//!    reconfig — that's the x → x+1 boundary.
//!
//! 6. In epoch x+1 the test asserts:
//!      - dice roll commits iff current randomness DKG = Finished;
//!      - the on-chain `PerEpochEncryptionKey` is Some iff any chunky DKG
//!        completed up to and including epoch x (i.e. prior or current
//!        chunky DKG = Finished);
//!      - the chain commits plain transactions.
//!
//! All tests are #[ignore]'d to keep them out of CI. Run on-demand via:
//!   cargo nextest run -p smoke-test --run-ignored=ignored-only \
//!     -E 'test(/watchdog_matrix_/)'

use crate::{smoke_test_environment::SwarmBuilder, utils::get_current_consensus_config};
use aptos::{common::types::GasOptions, test::CliTestFramework};
use aptos_forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_move_cli::MemberId;
use aptos_rest_client::Client;
use aptos_types::{
    decryption::PerEpochEncryptionKeyResource,
    on_chain_config::{
        FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig,
    },
};
use move_core_types::{language_storage::CORE_CODE_ADDRESS, move_resource::MoveStructType};
use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::Duration};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DkgOutcome {
    NotTriggered,
    Aborted,
    Finished,
}

use DkgOutcome::*;

/// epoch_duration_secs has to comfortably cover, in epoch x-1: dice module
/// publish + governance script submission + chain catch-up. 40s is safe.
const EPOCH_DURATION_SECS: u64 = 40;
/// Watchdog grace period: how long after a stuck reconfig the watchdog
/// force-finalizes. 30s is enough for a stuck DKG to be obvious.
const WATCHDOG_GRACE_SECS: u64 = 30;

#[derive(Copy, Clone, Debug)]
struct MatrixCase {
    prior_rdkg: DkgOutcome,
    prior_cdkg: DkgOutcome,
    cur_rdkg: DkgOutcome,
    cur_cdkg: DkgOutcome,
}

impl MatrixCase {
    fn new(prior_r: DkgOutcome, prior_c: DkgOutcome, cur_r: DkgOutcome, cur_c: DkgOutcome) -> Self {
        if prior_r == NotTriggered {
            assert_eq!(
                prior_c, NotTriggered,
                "impossible case: chunky DKG cannot be triggered when randomness DKG is not"
            );
        }
        assert_ne!(cur_r, NotTriggered, "current randomness DKG must be triggered per matrix");
        assert_ne!(cur_c, NotTriggered, "current chunky DKG must be triggered per matrix");
        Self {
            prior_rdkg: prior_r,
            prior_cdkg: prior_c,
            cur_rdkg: cur_r,
            cur_cdkg: cur_c,
        }
    }
}

// Failpoint identifiers. The randomness one is a closure-form variant added
// specifically so tests can use action="return" to make process_dkg_start_event
// return early without panicking — the existing "dkg::process_dkg_start_event"
// has no closure form.
const FAILPOINT_RANDOMNESS_DKG_ABORT: &str = "dkg::process_dkg_start_event_test_skip";
const FAILPOINT_CHUNKY_DKG_ABORT: &str = "chunky_dkg::process_dkg_start_event";

async fn set_failpoint(swarm: &LocalSwarm, name: &str, action: &str) {
    for validator in swarm.validators() {
        validator
            .rest_client()
            .set_failpoint(name.to_string(), action.to_string())
            .await
            .unwrap_or_else(|e| panic!("failed to set failpoint {name}={action}: {e}"));
    }
}

async fn arm_failpoint(swarm: &LocalSwarm, name: &str) {
    set_failpoint(swarm, name, "return").await;
}

async fn disarm_failpoint(swarm: &LocalSwarm, name: &str) {
    // failpoints-rs uses action="off" to disable an existing failpoint.
    set_failpoint(swarm, name, "off").await;
}

/// Build the smoke-test swarm with genesis features matching the prior epoch
/// state. The epoch-timeout watchdog is armed from genesis (grace
/// WATCHDOG_GRACE_SECS).
async fn build_swarm_for_prior(
    num_validators: usize,
    prior_rdkg_triggered: bool,
    prior_cdkg_triggered: bool,
) -> (LocalSwarm, CliTestFramework, usize) {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(num_validators)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config.state_sync.state_sync_driver.enable_auto_bootstrapping = true;
            config.state_sync.state_sync_driver.max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = EPOCH_DURATION_SECS;
            conf.allow_new_validators = true;
            conf.epoch_timeout_grace_period_secs_override = Some(WATCHDOG_GRACE_SECS);

            if prior_rdkg_triggered {
                conf.consensus_config.enable_validator_txns();
                conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            } else {
                conf.consensus_config.disable_validator_txns();
                conf.randomness_config_override = Some(OnChainRandomnessConfig::default_disabled());
            }

            if prior_cdkg_triggered {
                conf.chunky_dkg_config_override =
                    Some(OnChainChunkyDKGConfig::default_enabled());
                let mut features = Features::default();
                features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
                conf.initial_features_override = Some(features);
            }
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);
    (swarm, cli, root_idx)
}

/// Single governance script: buffers whatever needs to flip from off to on
/// for epoch x, then calls `aptos_governance::reconfigure`. The reconfigure
/// auto-routes (sync via finish vs async via try_start vs async via
/// try_start_with_chunky_dkg) based on what's currently enabled.
async fn run_gov_to_enable_current_features(
    cli: &CliTestFramework,
    root_idx: usize,
    client: &Client,
    enable_validator_txns: bool,
    enable_randomness: bool,
    enable_chunky_v1: bool,
    enable_encrypted_transactions: bool,
) {
    let mut imports = String::new();
    let mut body = String::new();

    if enable_validator_txns {
        let mut config = get_current_consensus_config(client).await;
        config.enable_validator_txns();
        let bytes = bcs::to_bytes(&config).unwrap();
        imports.push_str("    use aptos_framework::consensus_config;\n");
        body.push_str(&format!(
            "        let consensus_bytes = vector{:?};\n        consensus_config::set_for_next_epoch(&framework_signer, consensus_bytes);\n",
            bytes
        ));
    }

    if enable_randomness {
        imports.push_str("    use aptos_framework::randomness_config;\n");
        imports.push_str("    use aptos_std::fixed_point64;\n");
        body.push_str(
            "        let r_cfg = randomness_config::new_v1(\n            fixed_point64::create_from_rational(1, 2),\n            fixed_point64::create_from_rational(2, 3),\n        );\n        randomness_config::set_for_next_epoch(&framework_signer, r_cfg);\n",
        );
    }

    if enable_chunky_v1 {
        imports.push_str("    use aptos_framework::chunky_dkg_config;\n");
        if !enable_randomness {
            imports.push_str("    use aptos_std::fixed_point64;\n");
        }
        body.push_str(
            "        let c_cfg = chunky_dkg_config::new_v1(\n            fixed_point64::create_from_rational(1, 2),\n            fixed_point64::create_from_rational(2, 3),\n        );\n        chunky_dkg_config::set_for_next_epoch(&framework_signer, c_cfg);\n",
        );
    }

    if enable_encrypted_transactions {
        imports.push_str("    use aptos_framework::features;\n");
        body.push_str(
            "        features::change_feature_flags_for_next_epoch(&framework_signer, vector[108], vector[]);\n",
        );
    }

    let script = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;
{}
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
{}
        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        imports, body
    );
    cli.run_script(root_idx, &script)
        .await
        .expect("governance script failed");
}

/// Wait until `client` sees an epoch ≥ `target`. Logs progress every 5s.
async fn wait_for_epoch(client: &Client, target: u64, time_limit_secs: u64) {
    let timer = tokio::time::Instant::now();
    loop {
        let info = client
            .get_ledger_information()
            .await
            .expect("ledger info")
            .into_inner();
        info!(
            "waiting for epoch {}: now={} elapsed={}s",
            target,
            info.epoch,
            timer.elapsed().as_secs()
        );
        if info.epoch >= target {
            return;
        }
        if timer.elapsed().as_secs() >= time_limit_secs {
            panic!(
                "timed out waiting for epoch {} (current {}, limit {}s)",
                target, info.epoch, time_limit_secs
            );
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// ----------------------- Functional checks -----------------------

/// Publish the move-example `on_chain_dice` module from the workspace.
async fn publish_dice_module(cli: &mut CliTestFramework, publisher_idx: usize) {
    cli.init_move_dir();
    let mut package_addresses = BTreeMap::new();
    package_addresses.insert("module_owner", "_");

    cli.init_package(
        "OnChainDice".to_string(),
        package_addresses,
        Some(CliTestFramework::aptos_framework_dir()),
    )
    .await
    .unwrap();

    let content =
        include_str!("../../../../aptos-move/move-examples/on_chain_dice/sources/dice.move")
            .to_string();
    cli.add_file_in_package("sources/dice.move", content);

    cli.wait_for_account(publisher_idx).await.unwrap();
    let mut named = BTreeMap::new();
    let account = cli.account_id(publisher_idx).to_string();
    named.insert("module_owner", account.as_str());
    cli.publish_package(publisher_idx, None, named, None)
        .await
        .unwrap();
}

/// Call `0x<publisher>::dice::roll`. Succeeds iff `expect_success`.
async fn roll_dice_and_expect(
    cli: &mut CliTestFramework,
    publisher_idx: usize,
    expect_success: bool,
) {
    let account = cli.account_id(publisher_idx).to_hex_literal();
    let roll = MemberId::from_str(&format!("{}::dice::roll", account)).unwrap();
    let gas_options = GasOptions {
        gas_unit_price: Some(100),
        max_gas: Some(10_000),
        expiration_secs: 60,
    };
    let result = cli
        .run_function(publisher_idx, Some(gas_options), roll, vec![], vec![])
        .await;
    match (result, expect_success) {
        (Ok(s), true) => info!("dice::roll committed as expected: {:?}", s.transaction_hash),
        (Err(e), false) => info!("dice::roll failed as expected (randomness unavailable): {}", e),
        (Ok(s), false) => panic!(
            "dice::roll unexpectedly committed when randomness DKG aborted: {:?}",
            s
        ),
        (Err(e), true) => panic!(
            "dice::roll unexpectedly failed when randomness DKG finished: {}",
            e
        ),
    }
}

async fn assert_encryption_key_presence(client: &Client, expect_some: bool) {
    let tag = PerEpochEncryptionKeyResource::struct_tag();
    let resource = client
        .get_account_resource_bcs::<PerEpochEncryptionKeyResource>(
            CORE_CODE_ADDRESS,
            &tag.to_canonical_string(),
        )
        .await
        .expect("PerEpochEncryptionKeyResource")
        .into_inner();
    let actual_some = resource.encryption_key.is_some();
    assert_eq!(
        actual_some, expect_some,
        "encryption_key.is_some() = {}, expected {}",
        actual_some, expect_some
    );
}

async fn assert_chain_progresses(client: &Client) {
    let v_before = client
        .get_ledger_information()
        .await
        .expect("ledger info")
        .into_inner()
        .version;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let v_after = client
        .get_ledger_information()
        .await
        .expect("ledger info")
        .into_inner()
        .version;
    assert!(
        v_after > v_before,
        "chain did not progress: version {} → {}",
        v_before,
        v_after
    );
}

// ----------------------- Case runner -----------------------

async fn run_matrix_case(case: MatrixCase) {
    info!("running matrix case: {:?}", case);
    let prior_rdkg_triggered = case.prior_rdkg != NotTriggered;
    let prior_cdkg_triggered = case.prior_cdkg != NotTriggered;

    let (swarm, mut cli, root_idx) =
        build_swarm_for_prior(4, prior_rdkg_triggered, prior_cdkg_triggered).await;
    let client = swarm.validators().nth(1).unwrap().rest_client();

    // Wait until the chain is in epoch 2 — that is epoch x-1 for this test.
    // With validator txns off (prior randomness DKG = NotTriggered case),
    // epoch 1 → 2 is plain block_prologue (no DKG); with validator txns on,
    // a randomness DKG ran but completed naturally (no failpoints armed yet).
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(EPOCH_DURATION_SECS * 4))
        .await
        .expect("waiting for epoch 2");

    publish_dice_module(&mut cli, root_idx).await;

    let epoch_x_minus_1 = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "at epoch {} (x-1); arming prior-outcome failpoints",
        epoch_x_minus_1
    );

    if case.prior_rdkg == Aborted {
        arm_failpoint(&swarm, FAILPOINT_RANDOMNESS_DKG_ABORT).await;
    }
    if case.prior_cdkg == Aborted {
        arm_failpoint(&swarm, FAILPOINT_CHUNKY_DKG_ABORT).await;
    }

    info!("running governance script to flip features on for x and trigger x-1 → x reconfig");
    run_gov_to_enable_current_features(
        &cli,
        root_idx,
        &client,
        /* enable_validator_txns       */ !prior_rdkg_triggered,
        /* enable_randomness           */ !prior_rdkg_triggered,
        /* enable_chunky_v1            */ !prior_cdkg_triggered,
        /* enable_encrypted_transactions */ !prior_cdkg_triggered,
    )
    .await;

    // Wait for the gov-triggered reconfig to land us in epoch x.
    let epoch_x = epoch_x_minus_1 + 1;
    wait_for_epoch(
        &client,
        epoch_x,
        EPOCH_DURATION_SECS + WATCHDOG_GRACE_SECS + 60,
    )
    .await;
    info!("at epoch {} (x); adjusting failpoints for current outcomes", epoch_x);

    if case.cur_rdkg == Aborted {
        arm_failpoint(&swarm, FAILPOINT_RANDOMNESS_DKG_ABORT).await;
    } else {
        disarm_failpoint(&swarm, FAILPOINT_RANDOMNESS_DKG_ABORT).await;
    }
    if case.cur_cdkg == Aborted {
        arm_failpoint(&swarm, FAILPOINT_CHUNKY_DKG_ABORT).await;
    } else {
        disarm_failpoint(&swarm, FAILPOINT_CHUNKY_DKG_ABORT).await;
    }

    // Wait for the natural end-of-x reconfig → x+1.
    let epoch_x_plus_1 = epoch_x + 1;
    wait_for_epoch(
        &client,
        epoch_x_plus_1,
        EPOCH_DURATION_SECS * 2 + WATCHDOG_GRACE_SECS + 60,
    )
    .await;
    info!("at epoch {} (x+1); running functional assertions", epoch_x_plus_1);

    let expect_roll_success = case.cur_rdkg == Finished;
    let expect_key_some = case.prior_cdkg == Finished || case.cur_cdkg == Finished;

    assert_encryption_key_presence(&client, expect_key_some).await;
    roll_dice_and_expect(&mut cli, root_idx, expect_roll_success).await;
    assert_chain_progresses(&client).await;
    info!("matrix case {:?} OK", case);
}

// ----------------------- 28 #[ignore]'d tests -----------------------

macro_rules! matrix_test {
    ($name:ident, $pr:expr, $pc:expr, $cr:expr, $cc:expr) => {
        #[tokio::test]
        #[ignore]
        async fn $name() {
            run_matrix_case(MatrixCase::new($pr, $pc, $cr, $cc)).await;
        }
    };
}

// prior = (NotTriggered, NotTriggered) — 4 cases
matrix_test!(watchdog_matrix_nt_nt_f_f, NotTriggered, NotTriggered, Finished, Finished);
matrix_test!(watchdog_matrix_nt_nt_f_a, NotTriggered, NotTriggered, Finished, Aborted);
matrix_test!(watchdog_matrix_nt_nt_a_f, NotTriggered, NotTriggered, Aborted, Finished);
matrix_test!(watchdog_matrix_nt_nt_a_a, NotTriggered, NotTriggered, Aborted, Aborted);

// prior = (Aborted, NotTriggered) — 4 cases
matrix_test!(watchdog_matrix_a_nt_f_f, Aborted, NotTriggered, Finished, Finished);
matrix_test!(watchdog_matrix_a_nt_f_a, Aborted, NotTriggered, Finished, Aborted);
matrix_test!(watchdog_matrix_a_nt_a_f, Aborted, NotTriggered, Aborted, Finished);
matrix_test!(watchdog_matrix_a_nt_a_a, Aborted, NotTriggered, Aborted, Aborted);

// prior = (Finished, NotTriggered) — 4 cases
matrix_test!(watchdog_matrix_f_nt_f_f, Finished, NotTriggered, Finished, Finished);
matrix_test!(watchdog_matrix_f_nt_f_a, Finished, NotTriggered, Finished, Aborted);
matrix_test!(watchdog_matrix_f_nt_a_f, Finished, NotTriggered, Aborted, Finished);
matrix_test!(watchdog_matrix_f_nt_a_a, Finished, NotTriggered, Aborted, Aborted);

// prior = (Aborted, Aborted) — 4 cases
matrix_test!(watchdog_matrix_a_a_f_f, Aborted, Aborted, Finished, Finished);
matrix_test!(watchdog_matrix_a_a_f_a, Aborted, Aborted, Finished, Aborted);
matrix_test!(watchdog_matrix_a_a_a_f, Aborted, Aborted, Aborted, Finished);
matrix_test!(watchdog_matrix_a_a_a_a, Aborted, Aborted, Aborted, Aborted);

// prior = (Aborted, Finished) — 4 cases
matrix_test!(watchdog_matrix_a_f_f_f, Aborted, Finished, Finished, Finished);
matrix_test!(watchdog_matrix_a_f_f_a, Aborted, Finished, Finished, Aborted);
matrix_test!(watchdog_matrix_a_f_a_f, Aborted, Finished, Aborted, Finished);
matrix_test!(watchdog_matrix_a_f_a_a, Aborted, Finished, Aborted, Aborted);

// prior = (Finished, Aborted) — 4 cases
matrix_test!(watchdog_matrix_f_a_f_f, Finished, Aborted, Finished, Finished);
matrix_test!(watchdog_matrix_f_a_f_a, Finished, Aborted, Finished, Aborted);
matrix_test!(watchdog_matrix_f_a_a_f, Finished, Aborted, Aborted, Finished);
matrix_test!(watchdog_matrix_f_a_a_a, Finished, Aborted, Aborted, Aborted);

// prior = (Finished, Finished) — 4 cases
matrix_test!(watchdog_matrix_f_f_f_f, Finished, Finished, Finished, Finished);
matrix_test!(watchdog_matrix_f_f_f_a, Finished, Finished, Finished, Aborted);
matrix_test!(watchdog_matrix_f_f_a_f, Finished, Finished, Aborted, Finished);
matrix_test!(watchdog_matrix_f_f_a_a, Finished, Finished, Aborted, Aborted);
