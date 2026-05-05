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
//! We model this with the `consensus::send::any` failpoint in the validator
//! binary. Probabilistic delay (`X%delay(2000)` via the `fail` crate's
//! built-in syntax) makes a fraction of consensus sends arrive 2 seconds
//! late — well past the typical 1-second round timeout. Per-validator delay
//! percentage is set to that validator's measured `fp_7d_avg × 100`, so each
//! chronic/flaky validator fails proposals at exactly its mainnet rate.
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
}

impl Default for FailurePatternConfig {
    fn default() -> Self {
        Self {
            continuous_delay_ms: 2000,
            spike_at_offset_secs: 60..420,
            spike_pause_secs: 30..90,
            min_continuous_pct: 1,
        }
    }
}

/// Failpoint name used to inject failure on outbound consensus messages.
/// Matches `fail_point!("consensus::send::any", ...)` in `consensus/src/network.rs`.
const SEND_FAILPOINT: &str = "consensus::send::any";

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
        let mut healthy_peers: Vec<PeerId> = Vec::new();
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
                    healthy_peers.push(*peer_id);
                },
                AvailabilityClass::EpisodicSpike => {
                    by_class[2].1 += 1;
                    healthy_peers.push(*peer_id);
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
                    let pct = continuous_delay_pct(entry, self.config.min_continuous_pct);
                    let action = format!("{}%delay({})", pct, self.config.continuous_delay_ms);
                    if let Err(e) = client
                        .set_failpoint(SEND_FAILPOINT.to_string(), action.clone())
                        .await
                    {
                        warn!(
                            "set_failpoint chronic/flaky on {} ({}={}) failed: {:?}",
                            name, SEND_FAILPOINT, action, e
                        );
                    }
                },
            }
        }

        info!(
            "MainnetMirrorFailureTest: set continuous failpoints on {} chronic + {} flaky; \
             spawned {} spike tasks; emitter pool = {} validators",
            by_class[0].1,
            by_class[1].1,
            self.spike_tasks.lock().await.len(),
            healthy_peers.len(),
        );

        // Restrict the emitter to validators that aren't continuously perturbed.
        // Spike validators are mostly healthy until their event fires, so they
        // stay in the pool. Chronic/flaky are excluded — funding through a
        // 30%-delay endpoint risks tripping the emitter's 60s ledger-staleness
        // check during account funding.
        Ok(LoadDestination::Peers(healthy_peers))
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
            if let Err(e) = client
                .set_failpoint(SEND_FAILPOINT.to_string(), "off".to_string())
                .await
            {
                warn!("clear failpoint on {} failed: {:?}", name, e);
            }
        }

        info!(
            "MainnetMirrorFailureTest: aborted {} spike tasks; cleared {} failpoints",
            n,
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
/// snapshot's `fp_7d_avg` (a fraction in [0, 1]). Clamped to [min_pct, 99]
/// since the failpoint engine requires a non-zero, sub-100 percentage.
fn continuous_delay_pct(entry: &ValidatorEntry, min_pct: u32) -> u32 {
    let fp = entry.fp_7d_avg.unwrap_or(0.0);
    let raw = (fp * 100.0).round() as i64;
    raw.clamp(min_pct as i64, 99) as u64 as u32
}

/// Spawn the spike task for a single EpisodicSpike validator. Sleeps for a
/// random offset within `config.spike_at_offset_secs`, then sets the
/// `consensus::send::any` failpoint to `100%return` for `spike_pause_secs`,
/// then turns it off.
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

        if let Err(e) = client
            .set_failpoint(SEND_FAILPOINT.to_string(), "100%return".to_string())
            .await
        {
            warn!("spike set_failpoint on {} failed: {:?}", name, e);
            return;
        }
        tokio::time::sleep(pause).await;
        if let Err(e) = client
            .set_failpoint(SEND_FAILPOINT.to_string(), "off".to_string())
            .await
        {
            warn!("spike clear_failpoint on {} failed: {:?}", name, e);
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
        assert_eq!(continuous_delay_pct(&e, 1), 1);

        // Above 99%: clamps down.
        let e = entry(AvailabilityClass::StableChronic, Some(1.5));
        assert_eq!(continuous_delay_pct(&e, 1), 99);

        // Typical chronic and flaky values pass through.
        let e = entry(AvailabilityClass::StableChronic, Some(0.378));
        assert_eq!(continuous_delay_pct(&e, 1), 38);
        let e = entry(AvailabilityClass::OnlineButFlaky, Some(0.033));
        assert_eq!(continuous_delay_pct(&e, 1), 3);
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
