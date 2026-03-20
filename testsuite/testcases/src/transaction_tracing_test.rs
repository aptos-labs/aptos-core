// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::create_emitter_and_request;
use aptos_forge::{NetworkContextSynchronizer, NetworkTest, Node, NodeExt, Result, Test};
use aptos_rest_client::aptos_api_types::Transaction;
use async_trait::async_trait;
use log::info;
use rand::{rngs::StdRng, SeedableRng};
use std::{collections::HashSet, ops::DerefMut, time::Duration};

/// Number of unique senders to trace. The emitter assigns each account to one
/// validator via `index % num_validators`, and `maybe_start_trace` only fires
/// on the validator that receives the REST submission (duplicates from mempool
/// broadcast get non-Accepted status). To guarantee traces appear on all
/// validators we need enough senders that each validator has at least one.
/// With 2000 accounts across 4 validators, picking 50 senders from the latest
/// on-chain transactions virtually guarantees all validators are covered.
const NUM_TRACKED_SENDERS: usize = 50;
const SETUP_WAIT_SECS: u64 = 5;
const DEFAULT_TRAFFIC_DURATION_SECS: u64 = 300;

pub struct TransactionTracingTest;

impl Test for TransactionTracingTest {
    fn name(&self) -> &'static str {
        "transaction_tracing_test"
    }
}

#[async_trait]
impl NetworkTest for TransactionTracingTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();

        let all_validators = {
            ctx.swarm
                .read()
                .await
                .validators()
                .map(|v| v.peer_id())
                .collect::<Vec<_>>()
        };

        // Step 1: Create emitter and start the job.
        // start_job creates accounts (via bulk_create_accounts) and begins traffic.
        let emit_job_request = ctx.emit_job.clone();
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let (mut emitter, emit_job_request) =
            create_emitter_and_request(ctx.swarm.clone(), emit_job_request, &all_validators, rng)
                .await?;

        let root_account = ctx.swarm.read().await.chain_info().root_account;
        let phases = emit_job_request.get_num_phases();
        let mut job = emitter
            .start_job(root_account, emit_job_request, phases)
            .await?;
        info!("TxnTracing: emitter job started, accounts created and traffic flowing");

        // Step 2: Wait a few seconds for transactions to land on chain, then discover
        // sender addresses from the latest committed transactions. We pick enough
        // unique senders (NUM_TRACKED_SENDERS) to statistically guarantee coverage
        // across all validators.
        //
        // Why many senders? The emitter binds each account to one REST client
        // (validator) via `index % num_clients`. `maybe_start_trace` only fires on
        // `MempoolStatusCode::Accepted` — i.e., the validator that first receives
        // the txn via REST. Broadcast duplicates are not Accepted, so other
        // validators never create a trace. To see traces on every validator, we
        // need traced senders bound to each one.
        tokio::time::sleep(Duration::from_secs(5)).await;

        let rest_client = {
            ctx.swarm
                .read()
                .await
                .validators()
                .next()
                .expect("need at least one validator")
                .rest_client()
        };

        let txns = rest_client
            .get_transactions(None, Some(100))
            .await?
            .into_inner();

        let mut unique_senders = HashSet::new();
        for txn in &txns {
            if let Transaction::UserTransaction(user_txn) = txn {
                unique_senders.insert(user_txn.request.sender.inner().to_hex());
            }
        }

        let tracked_accounts: Vec<String> = unique_senders
            .into_iter()
            .take(NUM_TRACKED_SENDERS)
            .collect();

        info!(
            "TxnTracing: selected {} sender accounts to trace across {} validators",
            tracked_accounts.len(),
            all_validators.len(),
        );

        // Step 3: POST the tracing filter to each validator's admin service (write),
        // then verify via the inspection service (read-only).
        let client = reqwest::Client::new();
        let filter_json = serde_json::json!({
            "enabled": true,
            "sender_allowlist": tracked_accounts,
        });

        let validator_endpoints: Vec<_> = {
            let swarm = ctx.swarm.read().await;
            swarm
                .validators()
                .map(|v| {
                    let admin_port = v.config().admin_service.port;
                    let admin_url: reqwest::Url =
                        format!("http://localhost:{}/transaction_tracing", admin_port)
                            .parse()
                            .unwrap();
                    let mut inspect_url = v.inspection_service_endpoint();
                    inspect_url.set_path("transaction_tracing");
                    (v.peer_id(), admin_url, inspect_url)
                })
                .collect()
        };

        for (peer_id, admin_url, _) in &validator_endpoints {
            info!(
                "TxnTracing: POSTing filter to validator {} at {}",
                peer_id, admin_url
            );

            let resp = client.post(admin_url.clone()).json(&filter_json).send().await;

            match resp {
                Ok(r) => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    info!(
                        "TxnTracing: validator {} responded with status={}, body={}",
                        peer_id, status, body
                    );
                    assert!(
                        status.is_success(),
                        "Failed to set tracing filter on validator {}: {} {}",
                        peer_id,
                        status,
                        body
                    );
                },
                Err(e) => {
                    info!(
                        "TxnTracing: failed to reach validator {}: {}",
                        peer_id, e
                    );
                },
            }
        }

        // Step 4: Verify the filter was set by doing a GET on the inspection service
        {
            let (_, _, inspect_url) = &validator_endpoints[0];
            let resp = client.get(inspect_url.clone()).send().await?;
            let body = resp.text().await?;
            info!("TxnTracing: GET filter response: {}", body);
        }

        // Step 5: Let traffic continue for the remaining duration with tracing active.
        // Use global_duration if set (from --duration-secs), otherwise default to 480s.
        let traffic_duration = std::cmp::max(
            ctx.global_duration,
            Duration::from_secs(DEFAULT_TRAFFIC_DURATION_SECS),
        );
        let remaining = traffic_duration.saturating_sub(Duration::from_secs(SETUP_WAIT_SECS));
        info!(
            "TxnTracing: letting traffic run for {}s with tracing enabled",
            remaining.as_secs()
        );
        tokio::time::sleep(remaining).await;

        // Step 6: Stop the job and collect stats
        info!("TxnTracing: stopping emitter job");
        let stats = job.stop_job().await;
        let stats = stats.into_iter().next().expect("at least one phase");
        ctx.report
            .report_txn_stats(self.name().to_string(), &stats);

        info!("TxnTracing: traffic complete. Stats: {}", stats.rate());

        // Step 7: Print summary for Humio search
        info!("===== TRANSACTION TRACING TEST RESULTS =====");
        info!("TxnTracing: traced {} accounts", tracked_accounts.len());
        info!("TxnTracing: search Humio for 'TxnTrace' to find all traced transaction logs");
        info!("=============================================");

        Ok(())
    }
}
