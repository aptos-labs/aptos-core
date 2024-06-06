// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use anyhow::Context;
use aptos_forge::{NetworkContextSynchronizer, NetworkTest, NodeExt, Test};
use aptos_sdk::move_types::account_address::AccountAddress;
use async_trait::async_trait;
use std::{
    ops::DerefMut,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

pub struct TwinValidatorTest;

impl Test for TwinValidatorTest {
    fn name(&self) -> &'static str {
        "twin validator"
    }
}

impl NetworkLoadTest for TwinValidatorTest {}

#[async_trait]
impl NetworkTest for TwinValidatorTest {
    async fn run<'a>(&self, ctxa: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();
            let runtime = Runtime::new().unwrap();

            let all_validators_ids = ctx
                .swarm()
                .validators()
                .map(|v| v.peer_id())
                .collect::<Vec<_>>();
            let validator_count = all_validators_ids.len();
            let twin_count = 2;
            runtime.block_on(async {
                for i in 0..twin_count {
                    let main_id: AccountAddress = all_validators_ids[i];
                    let twin_id = all_validators_ids[i + validator_count - twin_count];
                    ctx.swarm()
                        .validator_mut(twin_id)
                        .unwrap()
                        .clear_storage()
                        .await
                        .context(format!(
                            "Error while clearing storage and stopping {twin_id}"
                        ))?;
                    let main_identity = ctx
                        .swarm()
                        .validator_mut(main_id)
                        .unwrap()
                        .get_identity()
                        .await
                        .context(format!("Error while getting identity for {main_id}"))?;
                    ctx.swarm()
                        .validator_mut(twin_id)
                        .unwrap()
                        .set_identity(main_identity)
                        .await
                        .context(format!("Error while setting identity for {twin_id}"))?;
                    ctx.swarm()
                        .validator_mut(twin_id)
                        .unwrap()
                        .start()
                        .await
                        .context(format!("Error while starting {twin_id}"))?;
                    ctx.swarm()
                        .validator_mut(twin_id)
                        .unwrap()
                        .wait_until_healthy(Instant::now() + Duration::from_secs(300))
                        .await
                        .context(format!("Error while waiting for {twin_id}"))?;
                }
                Ok::<(), anyhow::Error>(())
            })?;
        }
        <dyn NetworkLoadTest>::run(self, ctxa).await
    }
}
