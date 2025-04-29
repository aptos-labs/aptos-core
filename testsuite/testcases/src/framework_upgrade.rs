// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{batch_update, generate_traffic};
use anyhow::bail;
use aptos_forge::{
    NetworkContextSynchronizer, NetworkTest, Result, SwarmExt, Test, DEFAULT_ROOT_PRIV_KEY,
    FORGE_KEY_SEED,
};
use aptos_keygen::KeyGen;
use aptos_release_builder::ReleaseConfig;
use aptos_sdk::crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_temppath::TempPath;
use aptos_types::transaction::authenticator::AuthenticationKey;
use async_trait::async_trait;
use log::info;
use std::{ops::DerefMut, path::Path};
use tokio::{fs, time::Duration};

pub struct FrameworkUpgrade;

impl FrameworkUpgrade {
    pub const EPOCH_DURATION_SECS: u64 = 10;
}

impl Test for FrameworkUpgrade {
    fn name(&self) -> &'static str {
        "framework_upgrade::framework-upgrade"
    }
}

const RELEASE_YAML_PATH: &str = "aptos-move/aptos-release-builder/data";
const IGNORED_YAMLS: [&str; 2] = ["release.yaml", "example.yaml"];

fn is_release_yaml(path: &Path) -> bool {
    let basename = path.file_name().unwrap().to_str().unwrap();
    path.is_file()
        && path.extension().unwrap_or_default() == "yaml"
        && !IGNORED_YAMLS.contains(&basename)
}

#[async_trait]
impl NetworkTest for FrameworkUpgrade {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();

        let epoch_duration = Duration::from_secs(Self::EPOCH_DURATION_SECS);

        // Get the different versions we're testing with
        let (old_version, new_version) = {
            let mut versions = ctx.swarm.read().await.versions().collect::<Vec<_>>();
            versions.sort();
            if versions.len() != 2 {
                bail!("exactly two different versions needed to run compat test");
            }

            (versions[0].clone(), versions[1].clone())
        };

        let all_validators = {
            ctx.swarm
                .read()
                .await
                .validators()
                .map(|v| v.peer_id())
                .collect::<Vec<_>>()
        };

        let msg = format!(
            "Compatibility test results for {} ==> {} (PR)",
            old_version, new_version
        );
        info!("{}", msg);
        ctx.report.report_text(msg);

        // Update half the validators to latest version.
        let first_half = &all_validators[..all_validators.len() / 2];
        let msg = format!("Upgrade the nodes to version: {}", new_version);
        info!("{}", msg);
        ctx.report.report_text(msg);
        batch_update(ctx, first_half, &new_version).await?;

        // Generate some traffic
        let duration = Duration::from_secs(30);
        let txn_stat = generate_traffic(ctx, &all_validators, duration).await?;
        ctx.report.report_txn_stats(
            format!("{}::full-framework-upgrade", self.name()),
            &txn_stat,
        );

        {
            ctx.swarm.read().await.fork_check(epoch_duration).await?;
        }

        // Apply the framework release bundle.
        let root_key_path = TempPath::new();
        root_key_path.create_as_file()?;
        std::fs::write(
            root_key_path.path(),
            bcs::to_bytes(&Ed25519PrivateKey::try_from(
                hex::decode(DEFAULT_ROOT_PRIV_KEY)?.as_ref(),
            )?)?,
        )?;

        let starting_seed_in_decimal = i64::from_str_radix(FORGE_KEY_SEED, 16)?;

        // Initialize keyGen to get validator private keys. We uses the same seed in the test
        // driver as in the genesis script so that the validator keys are deterministic.
        let mut seed_slice = [0u8; 32];
        let seed_in_decimal = starting_seed_in_decimal;
        let seed_in_hex_string = format!("{seed_in_decimal:0>64x}");

        hex::decode_to_slice(seed_in_hex_string, &mut seed_slice)?;

        let mut keygen = KeyGen::from_seed(seed_slice);
        let validator_key = keygen.generate_ed25519_private_key();
        let validator_account =
            AuthenticationKey::ed25519(&validator_key.public_key()).account_address();

        let network_info = aptos_release_builder::validate::NetworkConfig {
            endpoint: ctx
                .swarm
                .read()
                .await
                .validators()
                .last()
                .unwrap()
                .rest_api_endpoint(),
            root_key_path: root_key_path.path().to_path_buf(),
            validator_account,
            validator_key,
            framework_git_rev: None,
        };

        network_info.mint_to_validator().await?;

        let release_config = aptos_release_builder::current_release_config();

        aptos_release_builder::validate::validate_config(
            release_config.clone(),
            network_info.clone(),
        )
        .await?;

        // Execute all the release yaml files
        let mut entries = fs::read_dir(RELEASE_YAML_PATH).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if is_release_yaml(&path) {
                let release_config = ReleaseConfig::parse(&fs::read_to_string(&path).await?)?;
                info!("Executing release yaml: {}", path.to_string_lossy());
                aptos_release_builder::validate::validate_config(
                    release_config.clone(),
                    network_info.clone(),
                )
                .await?;
            }
        }

        // Update the sequence number for the root account
        let root_account = { ctx.swarm.read().await.chain_info().root_account().address() };
        // Test the module publishing workflow
        {
            let chain_info = ctx.swarm.read().await.chain_info();
            let sequence_number = chain_info
                .rest_client()
                .get_account(root_account)
                .await
                .unwrap()
                .inner()
                .sequence_number;
            chain_info
                .root_account()
                .set_sequence_number(sequence_number);
        }

        // Generate some traffic
        let duration = Duration::from_secs(30);
        let txn_stat = generate_traffic(ctx, &all_validators, duration).await?;
        ctx.report.report_txn_stats(
            format!("{}::full-framework-upgrade", self.name()),
            &txn_stat,
        );

        {
            ctx.swarm.read().await.fork_check(epoch_duration).await?;
        }

        let msg = "5. check swarm health".to_string();
        info!("{}", msg);
        ctx.report.report_text(msg);
        {
            ctx.swarm.read().await.fork_check(epoch_duration).await?;
        }
        ctx.report.report_text(format!(
            "Compatibility test for {} ==> {} passed",
            old_version, new_version
        ));

        // Upgrade the rest
        let second_half = &all_validators[all_validators.len() / 2..];
        let msg = format!("Upgrade the remaining nodes to version: {}", new_version);
        info!("{}", msg);
        ctx.report.report_text(msg);
        batch_update(ctx, second_half, &new_version).await?;

        let duration = Duration::from_secs(30);
        let txn_stat = generate_traffic(ctx, &all_validators, duration).await?;
        ctx.report.report_txn_stats(
            format!("{}::full-framework-upgrade", self.name()),
            &txn_stat,
        );

        {
            ctx.swarm.read().await.fork_check(epoch_duration).await?;
        }

        Ok(())
    }
}
