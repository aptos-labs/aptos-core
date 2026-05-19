// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! 28-case matrix smoke tests for the epoch-timeout watchdog.
//!
//! Matrix axes (per @zjma's review of #19813):
//! - epoch x-1 (prior):  rDKG ∈ {NotTriggered, Aborted, Finished} × cDKG ∈ {NotTriggered, Aborted, Finished}
//! - epoch x   (current): rDKG ∈ {Aborted, Finished} × cDKG ∈ {Aborted, Finished}
//!
//! For every reachable (prior, current) combination, epoch x+1 must be reachable.
//! Two prior cells — (NT, Aborted) and (NT, Finished) — are impossible by framework
//! construction: `try_start_with_chunky_dkg` always starts BOTH rDKG and cDKG, so
//! cDKG cannot be triggered without rDKG. That leaves 7 valid prior states × 4 current
//! states = 28 cases.
//!
//! All cases are #[ignore]'d to keep them out of CI. Run on-demand via:
//!   cargo nextest run -p smoke-test -- --include-ignored watchdog_matrix
//!
//! This MVP file currently implements 3 representative cases:
//!   - NT_NT_F_A: matches the existing single-case test (prior sync reconfig, cDKG aborts in x).
//!   - F_F_F_F:   baseline; no aborts anywhere, watchdog never fires.
//!   - A_A_A_A:   stress; both DKGs abort in both x-1 and x.

use super::shadow_mode::create_swarm_with_dkg_only;
use crate::utils::get_on_chain_resource;
use aptos_forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::chunky_dkg::ChunkyDKGState,
    on_chain_config::{FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use std::{sync::Arc, time::Duration};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DkgOutcome {
    NotTriggered,
    Aborted,
    Finished,
}

use DkgOutcome::*;

const EPOCH_DURATION_SECS: u64 = 20;
const WATCHDOG_GRACE_SECS: u64 = 30;

/// Per-case parameters for the matrix.
#[derive(Copy, Clone, Debug)]
struct MatrixCase {
    prior_rdkg: DkgOutcome,
    prior_cdkg: DkgOutcome,
    cur_rdkg: DkgOutcome,
    cur_cdkg: DkgOutcome,
}

impl MatrixCase {
    fn new(prior_r: DkgOutcome, prior_c: DkgOutcome, cur_r: DkgOutcome, cur_c: DkgOutcome) -> Self {
        // Sanity: enforce the framework constraint (cDKG triggered ⇒ rDKG triggered).
        if prior_r == NotTriggered {
            assert_eq!(
                prior_c, NotTriggered,
                "impossible prior cell: cDKG triggered without rDKG"
            );
        }
        assert_ne!(cur_r, NotTriggered, "current rDKG must be triggered per matrix");
        assert_ne!(cur_c, NotTriggered, "current cDKG must be triggered per matrix");
        Self {
            prior_rdkg: prior_r,
            prior_cdkg: prior_c,
            cur_rdkg: cur_r,
            cur_cdkg: cur_c,
        }
    }

    /// True if the prior epoch produced NT,NT — used to pick "Class A" setup
    /// (genesis chunky off, governance with sync reconfig produces the NT prior).
    fn prior_is_nt_nt(&self) -> bool {
        self.prior_rdkg == NotTriggered && self.prior_cdkg == NotTriggered
    }
}

// ----------------------- Failpoint helpers -----------------------

const FP_RDKG: &str = "dkg::process_dkg_start_event_test_skip";
const FP_CDKG: &str = "chunky_dkg::process_dkg_start_event";

async fn set_failpoint_on_all(swarm: &LocalSwarm, name: &str, action: &str) {
    for v in swarm.validators() {
        v.rest_client()
            .set_failpoint(name.to_string(), action.to_string())
            .await
            .unwrap_or_else(|e| panic!("failed to set failpoint {name}: {e}"));
    }
}

async fn clear_failpoint_on_all(swarm: &LocalSwarm, name: &str) {
    // failpoints-rs accepts action="off" to disable.
    set_failpoint_on_all(swarm, name, "off").await;
}

// ----------------------- Governance scripts -----------------------

/// Class A governance: enable chunky V1 + watchdog + ENC in one shot, with a
/// trailing aptos_governance::reconfigure() that produces a synchronous
/// (no-DKG) epoch advance. Mirrors what enable_chunky_v1_and_watchdog does
/// in `epoch_timeout.rs`.
async fn gov_enable_all_with_sync_reconfig(
    cli: &aptos::test::CliTestFramework,
    root_idx: usize,
    watchdog_grace_secs: u64,
) {
    let script = format!(
        r#"
script {{
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::epoch_timeout_config;
    use aptos_framework::features;

    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let chunky_cfg = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3),
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, chunky_cfg);

        let timeout_cfg = epoch_timeout_config::new_with_grace_period({});
        epoch_timeout_config::set_for_next_epoch(&framework_signer, timeout_cfg);

        features::change_feature_flags_for_next_epoch(&framework_signer, vector[108], vector[]);

        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        watchdog_grace_secs
    );
    cli.run_script(root_idx, &script)
        .await
        .expect("governance: enable all + sync reconfig");
}

/// Class C governance: enable the watchdog ONLY, with sync reconfig. Used when
/// chunky is already on from genesis and we just need the watchdog up before
/// any abort can happen.
async fn gov_enable_watchdog_with_sync_reconfig(
    cli: &aptos::test::CliTestFramework,
    root_idx: usize,
    watchdog_grace_secs: u64,
) {
    let script = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::epoch_timeout_config;

    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let timeout_cfg = epoch_timeout_config::new_with_grace_period({});
        epoch_timeout_config::set_for_next_epoch(&framework_signer, timeout_cfg);

        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        watchdog_grace_secs
    );
    cli.run_script(root_idx, &script)
        .await
        .expect("governance: enable watchdog + sync reconfig");
}

// ----------------------- Swarm builders -----------------------

/// Class C swarm: chunky on from genesis + ENCRYPTED_TRANSACTIONS feature on.
/// Watchdog off at genesis (enabled via governance shortly after boot).
async fn create_swarm_class_c(
    num_validators: usize,
    epoch_duration_secs: u64,
) -> (
    aptos_forge::LocalSwarm,
    aptos::test::CliTestFramework,
    usize,
) {
    use crate::smoke_test_environment::SwarmBuilder;
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(num_validators)
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
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);
    (swarm, cli, root_idx)
}

// ----------------------- Functional check stubs -----------------------

/// Plain-txn progress: emit a small traffic burst and assert the chain commits
/// something. This is the minimal "chain is alive" check; richer randomness
/// and encryption assertions will be added when the matrix expands to 28.
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
        "chain did not progress: version {} -> {}",
        v_before,
        v_after
    );
}

// ----------------------- Case runner -----------------------

async fn run_matrix_case(case: MatrixCase) {
    info!("Running matrix case: {:?}", case);

    if case.prior_is_nt_nt() {
        run_class_a(case).await;
    } else {
        run_class_c(case).await;
    }
}

/// Class A: prior is (NT, NT). Use governance with sync reconfig during epoch
/// 2 to enable chunky+watchdog — that sync reconfig IS the x-1 → x transition
/// with no DKG. Then end of epoch 3 is the x → x+1 transition.
async fn run_class_a(case: MatrixCase) {
    assert!(case.prior_is_nt_nt());
    let (swarm, cli, root_idx) = create_swarm_with_dkg_only(4, EPOCH_DURATION_SECS).await;
    let client = swarm.validators().nth(1).unwrap().rest_client();

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(EPOCH_DURATION_SECS * 3))
        .await
        .expect("epoch 2");

    let epoch_before = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!("Class A: stable at epoch {}", epoch_before);

    // Failpoints for CURRENT epoch (x = epoch 3) outcomes.
    if case.cur_rdkg == Aborted {
        info!("Class A: enabling rDKG abort failpoint for current epoch");
        set_failpoint_on_all(&swarm, FP_RDKG, "return").await;
    }
    if case.cur_cdkg == Aborted {
        info!("Class A: enabling cDKG abort failpoint for current epoch");
        set_failpoint_on_all(&swarm, FP_CDKG, "return").await;
    }

    info!("Class A: governance enables chunky+watchdog+ENC (sync reconfig)");
    gov_enable_all_with_sync_reconfig(&cli, root_idx, WATCHDOG_GRACE_SECS).await;

    // Now we're in epoch 3 (= x). End of epoch 3 will produce the current state
    // via the failpoints set above.
    let target = epoch_before + 2;
    wait_for_epoch_with_logging(&client, target, EPOCH_DURATION_SECS, WATCHDOG_GRACE_SECS).await;

    assert_chain_progresses(&client).await;
    info!("Class A case {:?} OK", case);
}

/// Class C: prior involves chunky on in x-1. Genesis has chunky on; governance
/// in epoch 2 turns the watchdog on (sync reconfig → epoch 3). End of epoch 3
/// = x-1 → x transition produces prior state via failpoints set before the
/// transition; failpoints adjusted between epochs for current state.
async fn run_class_c(case: MatrixCase) {
    assert!(!case.prior_is_nt_nt(), "class C requires prior with at least one triggered");
    assert!(
        case.prior_rdkg != NotTriggered && case.prior_cdkg != NotTriggered,
        "MVP class C handles both prior r and c triggered; class B (prior cDKG=NT only) is post-MVP",
    );

    let (swarm, cli, root_idx) = create_swarm_class_c(4, EPOCH_DURATION_SECS).await;
    let client = swarm.validators().nth(1).unwrap().rest_client();

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(EPOCH_DURATION_SECS * 4))
        .await
        .expect("epoch 2");

    let epoch_at_gov = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!("Class C: stable at epoch {} (before watchdog gov)", epoch_at_gov);

    info!("Class C: governance enables watchdog (sync reconfig)");
    gov_enable_watchdog_with_sync_reconfig(&cli, root_idx, WATCHDOG_GRACE_SECS).await;

    // After sync reconfig, we're now in epoch_at_gov + 1. That epoch is x-1.
    // Set PRIOR-epoch failpoints before its natural reconfig.
    if case.prior_rdkg == Aborted {
        info!("Class C: prior rDKG=A — enabling rDKG abort failpoint");
        set_failpoint_on_all(&swarm, FP_RDKG, "return").await;
    }
    if case.prior_cdkg == Aborted {
        info!("Class C: prior cDKG=A — enabling cDKG abort failpoint");
        set_failpoint_on_all(&swarm, FP_CDKG, "return").await;
    }

    let prior_target = epoch_at_gov + 2; // sync gov advance + one natural transition
    wait_for_epoch_with_logging(&client, prior_target, EPOCH_DURATION_SECS, WATCHDOG_GRACE_SECS).await;

    // Now in epoch x = prior_target. Reconfigure failpoints for current outcomes.
    if case.cur_rdkg == Finished {
        clear_failpoint_on_all(&swarm, FP_RDKG).await;
    } else {
        set_failpoint_on_all(&swarm, FP_RDKG, "return").await;
    }
    if case.cur_cdkg == Finished {
        clear_failpoint_on_all(&swarm, FP_CDKG).await;
    } else {
        set_failpoint_on_all(&swarm, FP_CDKG, "return").await;
    }

    let current_target = prior_target + 1;
    wait_for_epoch_with_logging(&client, current_target, EPOCH_DURATION_SECS, WATCHDOG_GRACE_SECS).await;

    assert_chain_progresses(&client).await;
    info!("Class C case {:?} OK", case);
}

async fn wait_for_epoch_with_logging(
    client: &Client,
    target: u64,
    epoch_duration_secs: u64,
    grace_secs: u64,
) {
    let limit_secs = epoch_duration_secs * 2 + grace_secs + 60;
    let timer = tokio::time::Instant::now();
    loop {
        let info = client
            .get_ledger_information()
            .await
            .expect("ledger info")
            .into_inner();
        let dkg = get_on_chain_resource::<ChunkyDKGState>(client).await;
        info!(
            "waiting for epoch {}: now={} chunky_inprog={} chunky_done={} elapsed={}s",
            target,
            info.epoch,
            dkg.in_progress.is_some(),
            dkg.last_completed.is_some(),
            timer.elapsed().as_secs(),
        );
        if info.epoch >= target {
            return;
        }
        if timer.elapsed().as_secs() >= limit_secs {
            panic!(
                "timed out waiting for epoch {} (current {}, limit {}s)",
                target, info.epoch, limit_secs
            );
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// ----------------------- MVP tests (#[ignore]) -----------------------

/// Class A baseline — equivalent to the existing #19813 test (prior sync
/// reconfig, current rDKG finishes, current cDKG aborts via failpoint).
#[tokio::test]
#[ignore]
async fn watchdog_matrix_nt_nt_f_a() {
    run_matrix_case(MatrixCase::new(NotTriggered, NotTriggered, Finished, Aborted)).await;
}

/// Class C all-finished — no aborts anywhere; watchdog must not corrupt the
/// happy path.
#[tokio::test]
#[ignore]
async fn watchdog_matrix_f_f_f_f() {
    run_matrix_case(MatrixCase::new(Finished, Finished, Finished, Finished)).await;
}

/// Class C stress — both DKGs abort in x-1 AND x; watchdog must fire twice
/// in a row.
#[tokio::test]
#[ignore]
async fn watchdog_matrix_a_a_a_a() {
    run_matrix_case(MatrixCase::new(Aborted, Aborted, Aborted, Aborted)).await;
}
