// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Failure-pattern injection for the forge mainnet-mirror suite.
//!
//! Real mainnet has three observed failure classes (see
//! `mainnet-failure-pattern-2026-04-30.md`):
//!
//! - **StableChronic** (3-5 validators): persistent low-grade trouble — pod is
//!   alive, but a non-trivial fraction of its proposing rounds fail. Real
//!   mainnet hashport at fp_7d_avg=0.378 = ~1 missed proposal every ~3 min.
//! - **OnlineButFlaky** (5-10 validators): mostly healthy with occasional
//!   missed rounds (fp_7d_avg ~0.02-0.05).
//! - **EpisodicSpike** (5 validators): quiet 7d avg but had a one-shot bad
//!   event in the recent 30d window.
//!
//! Faithful modeling: real chronic operators stay UP — they don't drop
//! messages, they're just slow (slow disk, GC, network jitter). The proposal
//! IS sent, just late enough that voters time out the round before it arrives.
//!
//! We model this by setting probabilistic delay (`X%delay(2000)` via the
//! `fail` crate's built-in syntax) on a list of TARGETED consensus failpoints
//! covering the leader-side critical path (`broadcast_proposal`,
//! `broadcast_opt_proposal`, `vote`, `commit_vote`, `order_vote`) and the QS
//! batch-author critical path (`broadcast_batch`, `signed_batch_info`,
//! `proof_of_store`). Per-validator delay percentage = `fp_7d_avg × 100`.
//!
//! Importantly we do NOT set `consensus::send::any` — that broad failpoint
//! also fires inside `request_block` (state-sync block retrieval) and inside
//! every `send_rpc` / `broadcast`. Stacking 2-second delays on state-sync
//! requests on top of multi-region netem RTT cascaded into stalled state-sync
//! drivers in the 107-validator full run; ~20 chronic+flaky validators got
//! permanently stuck at constant version offsets. Restricting to the leader
//! and QS critical paths preserves the modeling intent ("chronic validators
//! propose / vote / disseminate batches slowly") without breaking sync.
//!
//! Failpoints (vs the previous chaos-mesh `loss` approach) avoid tc-qdisc
//! conflicts on pods that already carry inter-region netem rules, and apply
//! at the application layer for cleaner targeting.
//!
//! Requires `FORGE_ENABLE_FAILPOINTS=true` so the validator binary is built
//! with the `failpoints` cargo feature.

use crate::{
    mainnet_mirror::{AvailabilityClass, MainnetMirrorSnapshot, ValidatorEntry},
    multi_region_network_test::MultiRegionNetworkEmulationTest,
    LoadDestination, NetworkLoadTest,
};
use anyhow::ensure;
use aptos_forge::{
    NetworkContext, NetworkContextSynchronizer, NetworkTest, Result, SwarmExt, Test,
};
use aptos_logger::{info, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::PeerId;
use async_trait::async_trait;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{ops::Range, sync::Arc, time::Duration};
use tokio::{sync::Mutex, task::JoinHandle, time::Instant};

/// Thin wrapper that delegates `setup()` to the inner `MultiRegionNetworkEmulationTest`
/// but skips its `finish()` (which removes ~1100 PodNetworkChaos resources at
/// 106-validator scale and burns 3-5 min on the chaos-mesh controller). Forge's
/// pre-test cleanup already runs `kubectl delete networkchaos --all` on the
/// namespace at the start of every run, so end-of-test cleanup is redundant.
pub struct MultiRegionChaosNoCleanup(pub MultiRegionNetworkEmulationTest);

impl Test for MultiRegionChaosNoCleanup {
    fn name(&self) -> &'static str {
        "multi-region netem (skip cleanup)"
    }
}

#[async_trait]
impl NetworkLoadTest for MultiRegionChaosNoCleanup {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        self.0.setup(ctx).await
    }

    async fn finish<'a>(&self, _ctx: &mut NetworkContext<'a>) -> Result<()> {
        info!("MultiRegionChaosNoCleanup: skipping chaos teardown; next run's pre-cleanup will GC");
        Ok(())
    }
}

/// Tunable parameters for the failpoint-based failure injection.
///
/// Continuous delay magnitude for chronic/flaky validators is data-driven
/// (per-validator `fp_7d_avg × 100`%), so this struct only exposes the
/// fixed delay duration plus the spike-event timing.
#[derive(Debug, Clone)]
pub struct FailurePatternConfig {
    /// Delay (ms) applied to a fraction of consensus sends from chronic/flaky
    /// validators. Must exceed the round timeout (~1000ms by default) so the
    /// delayed message reliably misses the round and the round fails.
    pub continuous_delay_ms: u32,
    /// EpisodicSpike: when in the test the spike hits (seconds from start).
    pub spike_at_offset_secs: Range<u64>,
    /// EpisodicSpike: duration of the one-shot loss event.
    pub spike_pause_secs: Range<u64>,
    /// Floor on continuous-delay percentage applied to chronic/flaky
    /// validators with a missing or zero `fp_7d_avg` in the snapshot.
    pub min_continuous_pct: u32,
    /// Cap on continuous-delay percentage for the StableChronic class.
    /// Mainnet's "30% failed_proposals" is a per-leader-slot event count
    /// (~1 missed proposal per 3 min for hashport-class validators), not a
    /// 30% per-message dropout rate. Without this cap, mapping
    /// `fp_7d_avg × 100` directly produces e.g. `30%×delay(2s)` on every
    /// consensus message — order-of-magnitude over-modeling that cascades
    /// into leader-reputation suppression of healthy validators (observed
    /// run 25525952511, val 5/6/15 throttled despite being Healthy class).
    /// Flaky and Healthy classes are unaffected (flaky `fp_7d_avg` peaks
    /// around 5% on mainnet, well below this cap).
    pub max_chronic_continuous_pct: u32,
}

impl Default for FailurePatternConfig {
    fn default() -> Self {
        Self {
            continuous_delay_ms: 2000,
            // Earlier window so spike fires within the measured test window;
            // longer pause so val 4 leads 60-120+ rounds during the burst,
            // producing a clear chain-level signal.
            spike_at_offset_secs: 60..180,
            spike_pause_secs: 120..240,
            // Raised 1 -> 8 to bump flaky validators (bware/bitgo/sa-east at
            // raw 2.5-4.6%) up to a clearly-observable rate. Acts as a floor
            // for both flaky and chronic, but chronic's max=15 caps the top
            // so the effective range is [8, 15] for chronic, [8, 99] for
            // flaky. With X%return mechanism, this is the per-leader-round
            // failure rate.
            min_continuous_pct: 8,
            // Bumped 2 -> 15 so chronic apne1-0 returns 15% of leader-side
            // messages (matches mainnet's 30% failed_proposals magnitude
            // class while accounting for forge model differences). With
            // X%return mechanism, this is the per-leader-round failure rate.
            max_chronic_continuous_pct: 15,
        }
    }
}

/// Failpoint names used to inject failure on outbound consensus messages.
/// Each name matches a `fail_point!(...)` site in `consensus/src/network.rs`.
///
/// Covers ONLY the leader-side critical path (proposal + voting). QS batch
/// paths (broadcast_batch / signed_batch_info / proof_of_store) are
/// intentionally excluded: applying continuous X%delay(2s) to QS paths makes
/// every batch's lifecycle wait on slow validators, which over-models mainnet
/// — mainnet chronic validators do have elevated `failed_proposals_in_window`
/// but their QS contribution shows up as occasional misses (offline events),
/// not consistent 2s message delays. Run 25581268015 with QS paths included
/// produced p90=2000ms vs mainnet's ~750ms; removing them is the biggest
/// fidelity lever.
///
/// Also excludes the `consensus::send::any` superset failpoint because it
/// fires on `request_block` (state-sync block retrieval) — applying a
/// 2-second delay there cascades into stalled state-sync drivers under
/// multi-region netem.
///
/// Each chronic/flaky validator gets the SAME `X%delay(2000)` action applied
/// to all of these failpoints. Spike validators get `100%return` applied to
/// all of them for the spike duration.
const SEND_FAILPOINTS: &[&str] = &[
    // Leader-side: proposal broadcast + vote / commit_vote / order_vote sends.
    "consensus::send::broadcast_proposal",
    "consensus::send::broadcast_opt_proposal",
    "consensus::send::vote",
    "consensus::send::commit_vote",
    "consensus::send::order_vote",
];

pub struct MainnetMirrorFailureTest {
    snapshot: Arc<MainnetMirrorSnapshot>,
    config: FailurePatternConfig,
    /// Tokio JoinHandles for the spike tasks. Populated in `setup`, drained
    /// in `finish`. Mutex because setup/finish take `&self`.
    spike_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl MainnetMirrorFailureTest {
    pub fn new(snapshot: MainnetMirrorSnapshot) -> Self {
        Self::new_with_config(snapshot, FailurePatternConfig::default())
    }

    pub fn new_with_config(snapshot: MainnetMirrorSnapshot, config: FailurePatternConfig) -> Self {
        Self {
            snapshot: Arc::new(snapshot),
            config,
            spike_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Test for MainnetMirrorFailureTest {
    fn name(&self) -> &'static str {
        "mainnet mirror failure injection"
    }
}

#[async_trait]
impl NetworkLoadTest for MainnetMirrorFailureTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        let swarm = ctx.swarm.clone();
        let duration = ctx.global_duration;

        // Forge swarm.validators() returns by index, matching how the snapshot
        // is sorted (region asc, stake desc) — same ordering used by the
        // mainnet_mirror suite to align stake amounts and chaos regions.
        let validator_peers: Vec<PeerId> = {
            let s = swarm.read().await;
            s.validators().map(|v| v.peer_id()).collect()
        };

        // Get rest clients in matching order so we can target failpoints by index.
        let validator_clients: Vec<(String, RestClient)> =
            { swarm.read().await.get_validator_clients_with_names() };
        ensure!(
            validator_clients.len() == validator_peers.len(),
            "validator client count {} doesn't match peer count {}",
            validator_clients.len(),
            validator_peers.len(),
        );

        let snap_validators = self.snapshot.validators();
        ensure!(
            validator_peers.len() <= snap_validators.len(),
            "swarm has {} validators but snapshot only has {}",
            validator_peers.len(),
            snap_validators.len(),
        );

        // Bucket validators by class. Chronic + flaky get a continuous failpoint
        // (`X%delay(2s)` matching their fp_7d_avg). Spike validators get a
        // separately-spawned task that sets+removes a `100%return` failpoint.
        // Build the emitter pool to include Healthy + Chronic + Flaky but
        // exclude Spike: spike's `100%return` for ~30s drops ALL outgoing
        // consensus messages, so any txn admitted to a spike validator's
        // mempool during its window sits stuck for the whole window — that
        // single-validator stall produces 30+ second P99 outliers without
        // contributing realistic mainnet P90 modeling.
        let mut non_spike_peers: Vec<PeerId> = Vec::new();
        let mut by_class = [
            (AvailabilityClass::StableChronic, 0usize),
            (AvailabilityClass::OnlineButFlaky, 0usize),
            (AvailabilityClass::EpisodicSpike, 0usize),
        ];

        for (i, peer_id) in validator_peers.iter().enumerate() {
            let entry = &snap_validators[i];
            let (name, client) = &validator_clients[i];
            match entry.availability {
                AvailabilityClass::Healthy => {
                    non_spike_peers.push(*peer_id);
                },
                AvailabilityClass::EpisodicSpike => {
                    by_class[2].1 += 1;
                    // Spike: spawn a task that flips the failpoint on at offset,
                    // off after pause. Use 100%return for the spike duration to
                    // model a brief, severe outage event.
                    let handle = spawn_spike_task(
                        name.clone(),
                        client.clone(),
                        self.config.clone(),
                        duration,
                    );
                    self.spike_tasks.lock().await.push(handle);
                },
                AvailabilityClass::StableChronic | AvailabilityClass::OnlineButFlaky => {
                    let class_idx = if entry.availability == AvailabilityClass::StableChronic {
                        0
                    } else {
                        1
                    };
                    by_class[class_idx].1 += 1;
                    let max_pct = if entry.availability == AvailabilityClass::StableChronic {
                        self.config.max_chronic_continuous_pct
                    } else {
                        99
                    };
                    let pct = continuous_delay_pct(
                        entry,
                        self.config.min_continuous_pct,
                        max_pct,
                    );
                    // Use `return` (drop the message) instead of `delay(2000)` so the
                    // failpoint reliably fails the round when triggered. With delay=2000ms,
                    // observed runs showed flaky validators recording 0 failures because
                    // the 2s delay didn't always exceed round timeout — the leader still
                    // managed to commit. Dropping the message guarantees the round fails
                    // for the leader at exactly the configured pct.
                    let action = format!("{}%return", pct);
                    for fp in SEND_FAILPOINTS {
                        if let Err(e) = client.set_failpoint(fp.to_string(), action.clone()).await {
                            warn!(
                                "set_failpoint chronic/flaky on {} ({}={}) failed: {:?}",
                                name, fp, action, e
                            );
                        }
                    }
                    non_spike_peers.push(*peer_id);
                },
            }
        }

        info!(
            "MainnetMirrorFailureTest: set continuous failpoints on {} chronic + {} flaky; \
             spawned {} spike tasks; emitter pool = {} non-spike validators",
            by_class[0].1,
            by_class[1].1,
            self.spike_tasks.lock().await.len(),
            non_spike_peers.len(),
        );

        // Include Healthy + Chronic + Flaky in the emit pool to exercise the
        // chronic-validator-as-batch-author path that drives mainnet P90 (chronic
        // broadcasts hit `consensus::send::broadcast_batch` /
        // `consensus::send::signed_batch_info` / `consensus::send::proof_of_store`
        // delays from this test, slowing QS proof formation). Spike is excluded
        // — see comment above about 30s catastrophic-tail outliers.
        Ok(LoadDestination::Peers(non_spike_peers))
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<()> {
        // Abort spike tasks (spike pause may not have run remove_failpoint).
        let mut tasks = self.spike_tasks.lock().await;
        let n = tasks.len();
        for task in tasks.drain(..) {
            task.abort();
        }

        // Best-effort: clear the chronic/flaky failpoints. Validators are about
        // to be torn down anyway, but explicit clear avoids surprising any
        // downstream logic that inspects pod state during teardown.
        let validator_clients: Vec<(String, RestClient)> =
            { ctx.swarm.read().await.get_validator_clients_with_names() };
        for (name, client) in &validator_clients {
            for fp in SEND_FAILPOINTS {
                if let Err(e) = client
                    .set_failpoint(fp.to_string(), "off".to_string())
                    .await
                {
                    warn!("clear failpoint {} on {} failed: {:?}", fp, name, e);
                }
            }
        }

        info!(
            "MainnetMirrorFailureTest: aborted {} spike tasks; cleared {} failpoints across {} validators",
            n,
            SEND_FAILPOINTS.len(),
            validator_clients.len()
        );
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for MainnetMirrorFailureTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

/// Compute the percentage to inject for chronic/flaky validators from the
/// snapshot's `fp_7d_avg` (a fraction in [0, 1]). Clamped to
/// [`min_pct`, `max_pct`]. Caller passes `max_pct = 99` for flaky and a
/// lower value (e.g. 8) for chronic so that mainnet's chronic event-rate
/// failure isn't translated into a per-message dropout rate that would
/// over-model by an order of magnitude.
fn continuous_delay_pct(entry: &ValidatorEntry, min_pct: u32, max_pct: u32) -> u32 {
    let fp = entry.fp_7d_avg.unwrap_or(0.0);
    let raw = (fp * 100.0).round() as i64;
    raw.clamp(min_pct as i64, max_pct as i64) as u64 as u32
}

/// Spawn the spike task for a single EpisodicSpike validator. Sleeps for a
/// random offset within `config.spike_at_offset_secs`, then sets each of the
/// `SEND_FAILPOINTS` to `100%return` for `spike_pause_secs`, then turns them
/// off. Targeting the leader + QS critical paths (rather than
/// `consensus::send::any`) keeps state-sync working through the spike window.
fn spawn_spike_task(
    name: String,
    client: RestClient,
    config: FailurePatternConfig,
    duration: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut rng = SmallRng::from_entropy();
        let start = Instant::now();
        let when = rand_dur(&mut rng, &config.spike_at_offset_secs);
        if when >= duration {
            return;
        }
        tokio::time::sleep(when).await;
        let remaining = duration.saturating_sub(start.elapsed());
        if remaining == Duration::ZERO {
            return;
        }
        let pause = rand_dur(&mut rng, &config.spike_pause_secs).min(remaining);

        let mut any_set = false;
        for fp in SEND_FAILPOINTS {
            if let Err(e) = client
                .set_failpoint(fp.to_string(), "100%return".to_string())
                .await
            {
                warn!("spike set_failpoint {} on {} failed: {:?}", fp, name, e);
            } else {
                any_set = true;
            }
        }
        if !any_set {
            return;
        }
        tokio::time::sleep(pause).await;
        for fp in SEND_FAILPOINTS {
            if let Err(e) = client
                .set_failpoint(fp.to_string(), "off".to_string())
                .await
            {
                warn!("spike clear_failpoint {} on {} failed: {:?}", fp, name, e);
            }
        }
    })
}

fn rand_dur(rng: &mut SmallRng, range: &Range<u64>) -> Duration {
    let secs = if range.start >= range.end {
        range.start
    } else {
        rng.gen_range(range.start, range.end)
    };
    Duration::from_secs(secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mainnet_mirror::Region;

    fn entry(class: AvailabilityClass, fp: Option<f64>) -> ValidatorEntry {
        ValidatorEntry {
            peer_id: "0xabcd".into(),
            stake_octa: 1,
            region: Region::EuWest1,
            availability: class,
            fp_7d_avg: fp,
            fp_30d_max: None,
        }
    }

    #[test]
    fn rand_dur_respects_range() {
        let mut rng = SmallRng::from_entropy();
        for _ in 0..100 {
            let d = rand_dur(&mut rng, &(10..20));
            assert!(d >= Duration::from_secs(10));
            assert!(d < Duration::from_secs(20));
        }
    }

    #[test]
    fn rand_dur_handles_degenerate_range() {
        let mut rng = SmallRng::from_entropy();
        let d = rand_dur(&mut rng, &(5..5));
        assert_eq!(d, Duration::from_secs(5));
    }

    #[test]
    fn delay_pct_clamped_within_range() {
        // Below the floor: clamps up to min_pct.
        let e = entry(AvailabilityClass::OnlineButFlaky, Some(0.0));
        assert_eq!(continuous_delay_pct(&e, 1, 99), 1);

        // Above max_pct: clamps down.
        let e = entry(AvailabilityClass::StableChronic, Some(1.5));
        assert_eq!(continuous_delay_pct(&e, 1, 99), 99);
        // Chronic cap (8) clamps a typical mainnet chronic fp_7d_avg.
        let e = entry(AvailabilityClass::StableChronic, Some(0.378));
        assert_eq!(continuous_delay_pct(&e, 1, 8), 8);

        // Flaky's typical mainnet values pass through with max_pct=99.
        let e = entry(AvailabilityClass::OnlineButFlaky, Some(0.033));
        assert_eq!(continuous_delay_pct(&e, 1, 99), 3);
    }

    #[test]
    fn default_config_has_sensible_ranges() {
        let cfg = FailurePatternConfig::default();
        assert!(cfg.continuous_delay_ms >= 1500); // must exceed round timeout
        assert!(cfg.spike_at_offset_secs.start < cfg.spike_at_offset_secs.end);
        assert!(cfg.spike_pause_secs.start < cfg.spike_pause_secs.end);
        assert!(cfg.spike_at_offset_secs.end <= 480);
    }
}
