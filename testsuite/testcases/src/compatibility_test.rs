// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{batch_update_gradually, generate_traffic};
use anyhow::bail;
use velor_forge::{NetworkContextSynchronizer, NetworkTest, Result, SwarmExt, Test};
use async_trait::async_trait;
use log::info;
use std::ops::DerefMut;
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

#[async_trait]
impl NetworkTest for SimpleValidatorUpgrade {
    async fn run<'a>(&self, ctxa: NetworkContextSynchronizer<'a>) -> Result<()> {
        let upgrade_wait_for_healthy = true;
        let upgrade_node_delay = Duration::from_secs(20);
        let upgrade_max_wait = Duration::from_secs(40);

        let epoch_duration = Duration::from_secs(Self::EPOCH_DURATION_SECS);

        // Get the different versions we're testing with
        let (old_version, new_version) = {
            let mut versions = ctxa
                .ctx
                .lock()
                .await
                .swarm
                .read()
                .await
                .versions()
                .collect::<Vec<_>>();
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
        if ctxa
            .ctx
            .lock()
            .await
            .swarm
            .read()
            .await
            .validators()
            .count()
            < 4
        {
            bail!("compat test requires >= 4 validators");
        }
        let all_validators = ctxa
            .ctx
            .lock()
            .await
            .swarm
            .read()
            .await
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
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
        batch_update_gradually(
            ctxa.clone(),
            &[first_node],
            &new_version,
            upgrade_wait_for_healthy,
            upgrade_node_delay,
            upgrade_max_wait,
        )
        .await?;
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
        batch_update_gradually(
            ctxa.clone(),
            &first_batch,
            &new_version,
            upgrade_wait_for_healthy,
            upgrade_node_delay,
            upgrade_max_wait,
        )
        .await?;
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();

            // Generate some traffic
            let txn_stat_half = generate_traffic(ctx, &first_batch, duration).await?;
            ctx.report.report_txn_stats(
                format!("{}::half-validator-upgrade", self.name()),
                &txn_stat_half,
            );

            ctx.swarm.read().await.fork_check(epoch_duration).await?;

            // Update the second batch
            let msg = format!("4. upgrading second batch to new version: {}", new_version);
            info!("{}", msg);
            ctx.report.report_text(msg);
        }
        batch_update_gradually(
            ctxa.clone(),
            &second_batch,
            &new_version,
            upgrade_wait_for_healthy,
            upgrade_node_delay,
            upgrade_max_wait,
        )
        .await?;
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
            ctx.swarm.read().await.fork_check(epoch_duration).await?;
            ctx.report.report_text(format!(
                "Compatibility test for {} ==> {} passed",
                old_version, new_version
            ));
        }

        Ok(())
    }
}
