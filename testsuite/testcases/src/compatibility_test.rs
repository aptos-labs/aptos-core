// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{batch_update_gradually, create_emitter_and_request, generate_traffic};
use anyhow::bail;
use aptos_forge::{
    EmitJobRequest, NetworkContextSynchronizer, NetworkTest, Result, SwarmExt, Test, TxnEmitter,
    TxnStats, Version,
};
use aptos_logger::info;
// use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::{LocalAccount, PeerId};
use async_trait::async_trait;
use rand::SeedableRng;
use std::{
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::time::Duration;

pub struct SimpleValidatorUpgrade;

impl SimpleValidatorUpgrade {
    pub const EPOCH_DURATION_SECS: u64 = 30;
}

impl Test for SimpleValidatorUpgrade {
    fn name(&self) -> &'static str {
        "compatibility::simple-validator-upgrade"
    }
}

#[cfg(unused)]
async fn upgrade_task(
    // ctx: &mut NetworkContext<'_>,
    ctxa: NetworkContextSynchronizer<'_>,
    validators_to_update: &[PeerId],
    version: &Version,
    wait_until_healthy: bool,
    delay: Duration,
    max_wait: Duration,
    done: Arc<AtomicBool>,
) -> Result<()> {
    let result = batch_update_gradually(
        ctxa,
        validators_to_update,
        version,
        wait_until_healthy,
        delay,
        max_wait,
    )
    .await;
    done.store(true, Ordering::Relaxed);
    result
}
async fn stat_gather_task(
    emitter: TxnEmitter,
    emit_job_request: EmitJobRequest,
    source_account: Arc<LocalAccount>,
    upgrade_traffic_chunk_duration: Duration,
    done: Arc<AtomicBool>,
) -> Result<Option<TxnStats>> {
    let mut upgrade_stats = vec![];
    while !done.load(Ordering::Relaxed) {
        info!("stat_gather_task some traffic...");
        let upgrading_stats = emitter
            .clone()
            .emit_txn_for(
                source_account.clone(),
                emit_job_request.clone(),
                upgrade_traffic_chunk_duration,
            )
            .await?;
        info!("stat_gather_task some stats: {}", &upgrading_stats);
        upgrade_stats.push(upgrading_stats);
    }
    let statsum = upgrade_stats.into_iter().reduce(|a, b| &a + &b);
    Ok(statsum)
}

#[cfg(unused)]
fn traffic_task(
    ctxa: NetworkContextSynchronizer,
    nodes: &[PeerId],
    upgrade_done: Arc<AtomicBool>,
) -> Result<Option<TxnStats>> {
    let (emitter, emit_job_request, source_account) = {
        let mut ctx_locker = ctxa.ctx.lock().unwrap();
        let mut ctx = ctx_locker.deref_mut();
        // spawn_generate_traffic_setup(ctx, nodes)?
        let mut emit_job_request = ctx.emit_job.clone();
        let rng = SeedableRng::from_rng(ctx.core().rng()).unwrap();
        let swarm = ctx.swarm();
        let client_timeout = Duration::from_secs(30);

        let chain_info = swarm.chain_info();
        let transaction_factory = TransactionFactory::new(chain_info.chain_id);
        let emitter = TxnEmitter::new(transaction_factory, rng);

        emit_job_request =
            emit_job_request.rest_clients(swarm.get_clients_for_peers(nodes, client_timeout));
        let source_account = chain_info.root_account.clone();
        (emitter, emit_job_request, source_account)
    };
    // match create_emitter_and_request(ctx.swarm(), emit_job_request, nodes, rng) {
    //     Ok(parts) => parts,
    //     Err(err) => {
    //         stats_result = Err(err);
    //         return;
    //     }
    // };
    // let source_account = ctx.swarm().chain_info().root_account;
    let traffic_runtime = traffic_emitter_runtime()?;
    // let upgrade_joiner = handle.spawn(upgrade_task(ctx, validators_to_update, version, wait_until_healthy, delay, max_wait, upgrade_done.clone()));
    let upgrade_traffic_chunk_duration = Duration::from_secs(15);
    traffic_runtime.block_on(stat_gather_task(
        emitter,
        emit_job_request,
        source_account,
        upgrade_traffic_chunk_duration,
        upgrade_done.clone(),
    ))
}

fn upgrade_and_gather_stats(
    ctxa: NetworkContextSynchronizer,
    // upgrade args
    validators_to_update: &[PeerId],
    version: &Version,
    wait_until_healthy: bool,
    delay: Duration,
    max_wait: Duration,
    // traffic args
    nodes: &[PeerId],
) -> Result<Option<TxnStats>> {
    let upgrade_done = Arc::new(AtomicBool::new(false));
    let emitter_ctx = ctxa.clone();
    let mut stats_result: Result<Option<TxnStats>> = Ok(None);
    let mut upgrade_result: Result<()> = Ok(());
    tokio_scoped::scope(|scopev| {
        // emit trafic and gather stats
        scopev.spawn(async {
            info!("upgrade_and_gather_stats traffic thread start");
            let mut ctx_locker = emitter_ctx.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();
            let emit_job_request = ctx.emit_job.clone();
            let rng = SeedableRng::from_rng(ctx.core().rng()).unwrap();
            let (emitter, emit_job_request) =
                match create_emitter_and_request(ctx.swarm(), emit_job_request, nodes, rng) {
                    Ok(parts) => parts,
                    Err(err) => {
                        stats_result = Err(err);
                        return;
                    },
                };
            let source_account = ctx.swarm().chain_info().root_account;
            let upgrade_traffic_chunk_duration = Duration::from_secs(15);
            info!("upgrade_and_gather_stats traffic thread 1");
            stats_result = stat_gather_task(
                emitter,
                emit_job_request,
                source_account,
                upgrade_traffic_chunk_duration,
                upgrade_done.clone(),
            )
            .await;
            info!("upgrade_and_gather_stats traffic thread done");
        });
        // do upgrade
        scopev.spawn(async {
            info!("upgrade_and_gather_stats upgrade thread start");
            upgrade_result = batch_update_gradually(
                ctxa,
                validators_to_update,
                version,
                wait_until_healthy,
                delay,
                max_wait,
            )
            .await;
            info!("upgrade_and_gather_stats upgrade thread 1");
            upgrade_done.store(true, Ordering::Relaxed);
            info!("upgrade_and_gather_stats upgrade thread done");
        });
    });

    upgrade_result?;
    stats_result
}

#[async_trait]
impl NetworkTest for SimpleValidatorUpgrade {
    async fn run<'a>(&self, ctxa: NetworkContextSynchronizer<'a>) -> Result<()> {
        let upgrade_wait_for_healthy = true;
        let upgrade_node_delay = Duration::from_secs(10);
        let upgrade_max_wait = Duration::from_secs(40);

        let epoch_duration = Duration::from_secs(Self::EPOCH_DURATION_SECS);

        // Get the different versions we're testing with
        let (old_version, new_version) = {
            let mut versions = ctxa.ctx.lock().await.swarm().versions().collect::<Vec<_>>();
            versions.sort();
            if versions.len() != 2 {
                bail!("exactly two different versions needed to run compat test");
            }

            (versions[0].clone(), versions[1].clone())
        };

        let msg = format!(
            "Compatibility test results for {} ==> {} (PR)",
            old_version, new_version
        );
        info!("{}", msg);
        ctxa.report_text(msg).await;

        // Split the swarm into 2 parts
        if ctxa.ctx.lock().await.swarm().validators().count() < 4 {
            bail!("compat test requires >= 4 validators");
        }
        let all_validators = ctxa
            .ctx
            .lock()
            .await
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        // TODO: this is the "compat" test. Expand and refine to properly validate network2.
        // TODO: Ensure sustained TPS during upgrade. Slower upgrade rollout.
        let mut first_batch = all_validators.clone();
        let second_batch = first_batch.split_off(first_batch.len() / 2);
        let first_node = first_batch.pop().unwrap();
        let duration = Duration::from_secs(30);

        let msg = format!(
            "1. Check liveness of validators at old version: {}",
            old_version
        );
        info!("{}", msg);
        ctxa.report_text(msg).await;

        // Generate some traffic
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();
            let txn_stat_prior = generate_traffic(ctx, &all_validators, duration).await?;
            ctx.report
                .report_txn_stats(format!("{}::liveness-check", self.name()), &txn_stat_prior);
        }

        // Update the first Validator
        let msg = format!(
            "2. Upgrading first Validator to new version: {}",
            new_version
        );
        info!("{}", msg);
        ctxa.report_text(msg).await;
        // runtime.block_on(batch_update_gradually(ctx.swarm(), &[first_node], &new_version, upgrade_wait_for_healthy, upgrade_node_delay, upgrade_max_wait))?;
        let upgrade_stats = upgrade_and_gather_stats(
            ctxa.clone(),
            &[first_node],
            &new_version,
            upgrade_wait_for_healthy,
            upgrade_node_delay,
            upgrade_max_wait,
            &[first_node],
        )?;
        let upgrade_stats_sum = upgrade_stats.into_iter().reduce(|a, b| &a + &b);
        if let Some(upgrade_stats_sum) = upgrade_stats_sum {
            ctxa.ctx.lock().await.report.report_txn_stats(
                format!("{}::single-validator-upgrading", self.name()),
                &upgrade_stats_sum,
            );
        }

        // Generate some traffic
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();
            let txn_stat_one = generate_traffic(ctx, &[first_node], duration).await?;
            ctx.report.report_txn_stats(
                format!("{}::single-validator-upgrade", self.name()),
                &txn_stat_one,
            );

            // Update the rest of the first batch
            let msg = format!(
                "3. Upgrading rest of first batch to new version: {}",
                new_version
            );
            info!("{}", msg);
            ctx.report.report_text(msg);
        }

        // upgrade the rest of the first half
        let upgrade2_stats = upgrade_and_gather_stats(
            ctxa.clone(),
            &first_batch,
            &new_version,
            upgrade_wait_for_healthy,
            upgrade_node_delay,
            upgrade_max_wait,
            &first_batch,
        )?;
        let upgrade2_stats_sum = upgrade2_stats.into_iter().reduce(|a, b| &a + &b);
        if let Some(upgrade2_stats_sum) = upgrade2_stats_sum {
            ctxa.ctx.lock().await.report.report_txn_stats(
                format!("{}::half-validator-upgrading", self.name()),
                &upgrade2_stats_sum,
            );
        }
        // runtime.block_on(batch_update_gradually(ctxa.clone(), &first_batch, &new_version, upgrade_wait_for_healthy, upgrade_node_delay, upgrade_max_wait))?;
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();

            // Generate some traffic
            let txn_stat_half = generate_traffic(ctx, &first_batch, duration).await?;
            ctx.report.report_txn_stats(
                format!("{}::half-validator-upgrade", self.name()),
                &txn_stat_half,
            );

            ctx.swarm().fork_check(epoch_duration)?;

            // Update the second batch
            let msg = format!("4. upgrading second batch to new version: {}", new_version);
            info!("{}", msg);
            ctx.report.report_text(msg);
        }
        let upgrade3_stats = upgrade_and_gather_stats(
            ctxa.clone(),
            &second_batch,
            &new_version,
            upgrade_wait_for_healthy,
            upgrade_node_delay,
            upgrade_max_wait,
            &second_batch,
        )?;
        let upgrade3_stats_sum = upgrade3_stats.into_iter().reduce(|a, b| &a + &b);
        if let Some(upgrade3_stats_sum) = upgrade3_stats_sum {
            ctxa.ctx.lock().await.report.report_txn_stats(
                format!("{}::rest-validator-upgrading", self.name()),
                &upgrade3_stats_sum,
            );
        }
        // runtime.block_on(batch_update_gradually(ctxa.clone(), &second_batch, &new_version, upgrade_wait_for_healthy, upgrade_node_delay, upgrade_max_wait))?;
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();

            // Generate some traffic
            let txn_stat_all = generate_traffic(ctx, &second_batch, duration).await?;
            ctx.report.report_txn_stats(
                format!("{}::rest-validator-upgrade", self.name()),
                &txn_stat_all,
            );

            let msg = "5. check swarm health".to_string();
            info!("{}", msg);
            ctx.report.report_text(msg);
            ctx.swarm().fork_check(epoch_duration)?;
            ctx.report.report_text(format!(
                "Compatibility test for {} ==> {} passed",
                old_version, new_version
            ));
        }

        Ok(())
    }
}
