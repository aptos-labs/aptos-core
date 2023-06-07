// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{batch_update, generate_traffic};
use anyhow::bail;
use aptos_forge::{
    NetworkContext, NetworkTest, Result, SwarmExt, Test, DEFAULT_ROOT_PRIV_KEY, FORGE_KEY_SEED,
};
use aptos_keygen::KeyGen;
use aptos_logger::info;
use aptos_sdk::crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_temppath::TempPath;
use aptos_types::transaction::authenticator::AuthenticationKey;
use tokio::{runtime::Runtime, time::Duration};

pub struct FrameworkUpgrade;

impl Test for FrameworkUpgrade {
    fn name(&self) -> &'static str {
        "framework_upgrade::framework-upgrade"
    }
}

impl NetworkTest for FrameworkUpgrade {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let runtime = Runtime::new()?;

        // Get the different versions we're testing with
        let (old_version, new_version) = {
            let mut versions = ctx.swarm().versions().collect::<Vec<_>>();
            versions.sort();
            if versions.len() != 2 {
                bail!("exactly two different versions needed to run compat test");
            }

            (versions[0].clone(), versions[1].clone())
        };

        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let msg = format!(
            "Compatibility test results for {} ==> {} (PR)",
            old_version, new_version
        );
        info!("{}", msg);
        ctx.report.report_text(msg);

        // Update the validators to latest version.
        let msg = format!("Upgrade the nodes to version: {}", new_version);
        info!("{}", msg);
        ctx.report.report_text(msg);
        runtime.block_on(batch_update(ctx, &all_validators, &new_version))?;

        ctx.swarm().fork_check()?;

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
            AuthenticationKey::ed25519(&validator_key.public_key()).derived_address();

        let network_info = aptos_release_builder::validate::NetworkConfig {
            endpoint: ctx.swarm().validators().last().unwrap().rest_api_endpoint(),
            root_key_path: root_key_path.path().to_path_buf(),
            validator_account,
            validator_key,
            framework_git_rev: None,
        };

        runtime.block_on(network_info.mint_to_validator())?;

        let release_config = aptos_release_builder::current_release_config();

        runtime.block_on(aptos_release_builder::validate::validate_config(
            release_config.clone(),
            network_info,
        ))?;

        // Update the sequence number for the root account
        let root_account = ctx.swarm().chain_info().root_account().address();
        // Test the module publishing workflow
        *ctx.swarm()
            .chain_info()
            .root_account()
            .sequence_number_mut() = runtime
            .block_on(
                ctx.swarm()
                    .chain_info()
                    .rest_client()
                    .get_account(root_account),
            )
            .unwrap()
            .inner()
            .sequence_number;

        // Generate some traffic
        let duration = Duration::from_secs(30);
        let txn_stat = generate_traffic(ctx, &all_validators, duration)?;
        ctx.report.report_txn_stats(
            format!("{}::full-framework-upgrade", self.name()),
            &txn_stat,
        );

        ctx.swarm().fork_check()?;

        let msg = "5. check swarm health".to_string();
        info!("{}", msg);
        ctx.report.report_text(msg);
        ctx.swarm().fork_check()?;
        ctx.report.report_text(format!(
            "Compatibility test for {} ==> {} passed",
            old_version, new_version
        ));

        Ok(())
    }
}
