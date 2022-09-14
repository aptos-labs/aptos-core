// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos_logger::info;
use forge::{NetworkContext, NetworkTest, NodeExt, Test, Validator};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

pub struct TwinValidatorTest;

impl Test for TwinValidatorTest {
    fn name(&self) -> &'static str {
        "twin validator"
    }
}

impl NetworkLoadTest for TwinValidatorTest {}

impl NetworkTest for TwinValidatorTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
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
                let main_id = all_validators_ids[i];
                let twin_id = all_validators_ids[i + validator_count - twin_count];
                ctx.swarm().validator_mut(main_id).unwrap().stop().await;
                ctx.swarm()
                    .validator_mut(twin_id)
                    .unwrap()
                    .clear_storage()
                    .await;
                let main_identity: String = ctx
                    .swarm()
                    .validator_mut(main_id)
                    .unwrap()
                    .get_identity()
                    .await
                    .unwrap();
                ctx.swarm()
                    .validator_mut(twin_id)
                    .unwrap()
                    .set_identity(main_identity)
                    .await;
                ctx.swarm().validator_mut(twin_id).unwrap().start().await;
                ctx.swarm()
                    .validator_mut(twin_id)
                    .unwrap()
                    .wait_until_healthy(Instant::now() + Duration::from_secs(300))
                    .await;
                ctx.swarm().validator_mut(main_id).unwrap().start().await;
            }
        });
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
