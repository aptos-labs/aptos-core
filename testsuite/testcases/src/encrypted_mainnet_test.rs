// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{two_traffics_test::TwoTrafficsTest, LoadDestination, NetworkLoadTest};
use anyhow::{anyhow, Context};
use aptos::test::CliTestFramework;
use aptos_forge::{
    NetworkContext, NetworkContextSynchronizer, NetworkTest, Result, Swarm, SwarmExt, Test,
    TestReport,
};
use async_trait::async_trait;
use log::info;
use std::{sync::Arc, time::Duration};

/// Wraps `TwoTrafficsTest` to exercise the encrypted-txn path against the *mainnet* trusted setup,
/// which is provisioned for a fixed number of decryption rounds. The flow:
///
/// 1. Wait for the first epoch transition so chunky DKG produces a decryption key.
/// 2. Submit a governance script that extends `epoch_duration_secs` to 24h, pinning the chain to
///    a single working epoch for the rest of the run. The `reconfigure()` call also triggers an
///    immediate boundary so the new epoch length takes effect.
/// 3. Delegate to the inner `TwoTrafficsTest` for the full duration. Round count grows toward the
///    trusted-setup ceiling; past that point encrypted txns flip to `TrustedSetupExhausted` and
///    fail execution (submission stays healthy).
/// 4. After load completes, query the validator Prometheus counter
///    `aptos_consensus_decryption_pipeline_txns_count{category="trusted_setup_exhausted"}` and
///    assert it crossed `min_trusted_setup_exhausted` — i.e. exhaustion actually happened during
///    the run, not just at the very tail.
pub struct EncryptedMainnetTest {
    pub inner: TwoTrafficsTest,
    pub dkg_wait_timeout: Duration,
    pub new_epoch_duration_secs: u64,
    pub min_trusted_setup_exhausted: i64,
}

impl Test for EncryptedMainnetTest {
    fn name(&self) -> &'static str {
        "encrypted-mainnet-trusted-setup-test"
    }
}

#[async_trait]
impl NetworkLoadTest for EncryptedMainnetTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        // Step 1: wait for chunky DKG to complete. DKG runs at the first epoch boundary; we wait
        // for epoch >= 2 so the resulting decryption key is propagated and used.
        info!("Waiting for DKG (epoch >= 2)...");
        ctx.swarm
            .read()
            .await
            .wait_for_all_nodes_to_catchup_to_epoch(2, self.dkg_wait_timeout)
            .await
            .context("waiting for DKG epoch transition")?;
        info!("DKG complete.");

        // Step 2: extend epoch_duration_secs via governance so no further epoch changes happen
        // for the rest of the run. The reconfigure() call also triggers an immediate boundary
        // so the new value takes effect right away.
        let (rest_api_endpoint, faucet_endpoint) = {
            let swarm = ctx.swarm.read().await;
            let first_validator = swarm
                .validators()
                .next()
                .ok_or_else(|| anyhow!("no validators in swarm"))?;
            (
                first_validator.rest_api_endpoint(),
                "http://localhost:8081".parse().unwrap(),
            )
        };
        let mut cli = CliTestFramework::new(
            rest_api_endpoint,
            faucet_endpoint,
            /*num_cli_accounts=*/ 0,
        )
        .await;
        let root_cli_index = {
            let root_account = ctx.swarm.read().await.chain_info().root_account();
            cli.add_account_with_address_to_cli(
                root_account.private_key().clone(),
                root_account.address(),
            )
        };

        let new_epoch_micros: u64 = self
            .new_epoch_duration_secs
            .checked_mul(1_000_000)
            .ok_or_else(|| anyhow!("new_epoch_duration_secs overflow"))?;
        let extend_epoch_script = format!(
            r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::block;
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        block::update_epoch_interval_microsecs(&framework_signer, {});
        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
            new_epoch_micros
        );

        info!(
            "Submitting governance to extend epoch_duration to {}s",
            self.new_epoch_duration_secs
        );
        cli.run_script_with_default_framework(root_cli_index, &extend_epoch_script)
            .await
            .context("running extend-epoch governance script")?;
        info!("Epoch lock governance submitted.");

        // The governance txn advanced the root account's on-chain sequence number. The txn emitter
        // started after setup uses the same root account; if we don't resync the cached seq num,
        // the first emitted txn will fail SEQUENCE_NUMBER_TOO_OLD and the whole run aborts.
        {
            let chain_info = ctx.swarm.read().await.chain_info();
            let root_address = chain_info.root_account().address();
            let on_chain_seq = chain_info
                .rest_client()
                .get_account(root_address)
                .await
                .context("fetching root account to resync seq num")?
                .inner()
                .sequence_number;
            chain_info.root_account().set_sequence_number(on_chain_seq);
            info!("Root account seq num resynced to {}", on_chain_seq);
        }

        self.inner.setup(ctx).await
    }

    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        self.inner.test(swarm, report, duration).await
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<()> {
        // Assert that the trusted setup actually got exhausted during the run. Decryption pipeline
        // counts every encrypted txn in a post-ceiling block under
        // `category="trusted_setup_exhausted"` (consensus/src/pipeline/decryption_pipeline_builder.rs).
        let query = r#"sum(aptos_consensus_decryption_pipeline_txns_count{category="trusted_setup_exhausted"})"#;
        let exhausted = query_counter_sum(ctx.swarm.clone(), query).await?;
        let decrypted_query =
            r#"sum(aptos_consensus_decryption_pipeline_txns_count{category="decrypted"})"#;
        let decrypted = query_counter_sum(ctx.swarm.clone(), decrypted_query).await?;

        info!(
            "Decryption pipeline outcome: decrypted={}, trusted_setup_exhausted={}",
            decrypted, exhausted
        );

        // Sanity check: load must have actually exercised the encrypted path before exhaustion,
        // otherwise we'd be asserting the failure mode of a no-op.
        anyhow::ensure!(
            decrypted > 0,
            "no encrypted txns were ever decrypted (sum=0); test setup is wrong"
        );
        anyhow::ensure!(
            exhausted >= self.min_trusted_setup_exhausted,
            "expected trusted_setup_exhausted >= {} but got {} (decrypted={}); \
             the trusted-setup ceiling was not reached during the run — increase test duration \
             or the inner load",
            self.min_trusted_setup_exhausted,
            exhausted,
            decrypted,
        );

        self.inner.finish(ctx).await
    }
}

#[async_trait]
impl NetworkTest for EncryptedMainnetTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

async fn query_counter_sum(
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    query: &str,
) -> Result<i64> {
    let result = swarm.read().await.query_metrics(query, None, None).await?;
    let samples = result.as_instant().unwrap_or(&[]);
    Ok(samples
        .iter()
        .map(|s| s.sample().value().round() as i64)
        .sum())
}
