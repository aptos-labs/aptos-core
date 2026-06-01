// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::NetworkLoadTest;
use anyhow::anyhow;
use aptos_forge::{
    NetworkContextSynchronizer, NetworkTest, Result, Swarm, SwarmExt, Test, TestReport,
};
use aptos_rest_client::Client as RestClient;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS, dkg::chunky_dkg::ChunkyDKGState,
    on_chain_config::OnChainConfig,
};
use async_trait::async_trait;
use log::{info, warn};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

/// Stress test for Chunky DKG and encrypted-transaction load that drops the working quorum
/// *while a DKG is in flight*, so the epoch transition itself stalls and must recover.
///
/// Each cycle: poll a healthy validator until `0x1::chunky_dkg::ChunkyDKGState.in_progress`
/// becomes `Some` (a reconfiguration has started and chunky DKG is running for the next epoch),
/// then immediately blackhole a fixed set of `num_blackholed` validators at the network layer
/// (via the `network::send::any` / `network::recv::any` failpoints, which drop all peer-to-peer
/// traffic for every protocol without stopping the process). With more than f validators
/// blackholed the chain loses quorum, so the in-flight DKG cannot aggregate and the epoch
/// transition stalls (it cannot even force-end, since that also needs quorum). After the hold
/// window connectivity is restored, the DKG must resume, the new epoch must start, and the chain
/// must catch up.
pub struct ChunkyDkgQuorumLossTest {
    /// Number of validators to blackhole during each quorum-loss phase. Must exceed f (the BFT
    /// fault tolerance) so that the chain actually loses quorum.
    pub num_blackholed: usize,
    /// How long to hold the quorum-loss / DKG-stall phase before restoring connectivity.
    pub quorum_loss_secs: f32,
}

const SEND_FAILPOINT: &str = "network::send::any";
const RECV_FAILPOINT: &str = "network::recv::any";
/// How often to poll for the DKG-in-progress signal so we blackhole quorum mid-DKG.
const DKG_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Cap on how long to wait for the DKG to complete after connectivity is restored.
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(300);

impl Test for ChunkyDkgQuorumLossTest {
    fn name(&self) -> &'static str {
        "chunky dkg quorum loss test"
    }
}

/// Reads `0x1::chunky_dkg::ChunkyDKGState` and returns the target epoch of the in-progress DKG, if
/// any.
async fn chunky_dkg_in_progress_epoch(client: &RestClient) -> Result<Option<u64>> {
    let state = client
        .get_account_resource_bcs::<ChunkyDKGState>(
            CORE_CODE_ADDRESS,
            ChunkyDKGState::struct_tag().to_canonical_string().as_str(),
        )
        .await?
        .into_inner();
    Ok(state
        .in_progress
        .as_ref()
        .map(|session| session.target_epoch()))
}

/// Enable (drop all peer traffic) or disable the network blackhole failpoints on the given
/// validators. Failpoints are toggled over the REST API, which is independent of the peer-to-peer
/// network, so this works even while the targeted validators are blackholed.
async fn set_blackhole(validators: &[(String, RestClient)], enable: bool) -> Result<()> {
    let action = if enable { "return" } else { "off" };
    for (name, client) in validators {
        for failpoint in [SEND_FAILPOINT, RECV_FAILPOINT] {
            client
                .set_failpoint(failpoint.to_string(), action.to_string())
                .await
                .map_err(|e| {
                    anyhow!(
                        "set_failpoint {}={} on {} failed: {:?}",
                        failpoint,
                        action,
                        name,
                        e
                    )
                })?;
        }
    }
    Ok(())
}

#[async_trait]
impl NetworkLoadTest for ChunkyDkgQuorumLossTest {
    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        _report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        let validator_clients = { swarm.read().await.get_validator_clients_with_names() };
        let num_validators = validator_clients.len();

        // Blackhole a fixed set (the last `num_blackholed` validators) for the whole test, so it is
        // deterministic which validators are cut off. The first validator is never blackholed and
        // is used to poll for the DKG-in-progress signal.
        let blackholed: Vec<_> = validator_clients
            .iter()
            .skip(num_validators.saturating_sub(self.num_blackholed))
            .cloned()
            .collect();
        let (poll_name, poll_client) = validator_clients
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("no validators in swarm"))?;

        info!(
            "ChunkyDkgQuorumLossTest: {} validators total, polling {} for DKG-in-progress, \
             blackholing {} of them mid-DKG: {:?}",
            num_validators,
            poll_name,
            blackholed.len(),
            blackholed.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        );

        let start = Instant::now();
        let result = async {
            let mut cycle = 0;
            while start.elapsed() < duration {
                // 1. Wait for a chunky DKG to start (i.e. an epoch transition is under way),
                //    polling frequently so we can cut off quorum before the DKG aggregates.
                let mut target_epoch = None;
                while start.elapsed() < duration {
                    match chunky_dkg_in_progress_epoch(&poll_client).await {
                        Ok(Some(epoch)) => {
                            target_epoch = Some(epoch);
                            break;
                        },
                        Ok(None) => {},
                        Err(e) => {
                            warn!("Polling {} for ChunkyDKGState failed: {:?}", poll_name, e)
                        },
                    }
                    tokio::time::sleep(DKG_POLL_INTERVAL).await;
                }
                let Some(target_epoch) = target_epoch else {
                    break; // test duration elapsed while waiting for the next DKG
                };
                cycle += 1;

                // 2. Quorum-loss phase: blackhole the group mid-DKG. The in-flight chunky DKG can
                //    no longer aggregate and the epoch transition stalls.
                info!(
                    "Cycle {}: chunky DKG in progress for target epoch {}; blackholing {} \
                     validators for {}s to stall the DKG",
                    cycle,
                    target_epoch,
                    blackholed.len(),
                    self.quorum_loss_secs
                );
                set_blackhole(&blackholed, true).await?;
                tokio::time::sleep(Duration::from_secs_f32(self.quorum_loss_secs)).await;

                // 3. Restore connectivity: the DKG should resume and the epoch should advance.
                info!("Cycle {}: restoring connectivity; DKG should resume", cycle);
                set_blackhole(&blackholed, false).await?;

                // 4. Wait for the DKG to complete (in_progress clears) before looking for the next
                //    epoch transition, so we don't re-trigger on the same DKG.
                let recovery_start = Instant::now();
                loop {
                    match chunky_dkg_in_progress_epoch(&poll_client).await {
                        Ok(None) => {
                            info!("Cycle {}: DKG for epoch {} completed", cycle, target_epoch);
                            break;
                        },
                        Ok(Some(_)) => {},
                        Err(e) => warn!(
                            "Polling {} for ChunkyDKGState during recovery failed: {:?}",
                            poll_name, e
                        ),
                    }
                    if recovery_start.elapsed() > RECOVERY_TIMEOUT {
                        warn!(
                            "Cycle {}: DKG for epoch {} did not complete within {}s after restoring \
                             connectivity",
                            cycle,
                            target_epoch,
                            RECOVERY_TIMEOUT.as_secs()
                        );
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
            Ok(())
        }
        .await;

        // Safety net: always clear the failpoints, even if the loop errored mid-blackhole, so the
        // post-test catch-up checks run against a fully connected network.
        if let Err(e) = set_blackhole(&blackholed, false).await {
            warn!(
                "Failed to clear network blackhole failpoints at end of test: {:?}",
                e
            );
        }
        result
    }
}

#[async_trait]
impl NetworkTest for ChunkyDkgQuorumLossTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
