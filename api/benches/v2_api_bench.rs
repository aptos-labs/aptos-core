// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-core/aptos-core/blob/main/LICENSE

//! Performance benchmarks for the v2 API.
//!
//! Measures request latency and throughput for key v2 endpoints.
//! Run with: `cargo bench -p aptos-api`

use aptos_api::{
    context::Context,
    v2::{
        build_v2_router,
        context::{V2Config, V2Context},
    },
};
use aptos_api_test_context::new_test_context;
use aptos_config::config::NodeConfig;
use aptos_types::chain_id::ChainId;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Spin up a v2 server on a random port. Returns the base URL.
fn setup_v2_server(rt: &tokio::runtime::Runtime) -> String {
    rt.block_on(async {
        // Disable storage sharding so direct DB path is used (no indexer needed).
        let mut node_config = NodeConfig::default();
        node_config.storage.rocksdb_configs.enable_storage_sharding = false;

        let test_ctx = new_test_context("bench_v2".to_string(), node_config.clone(), false);

        // Build a fresh Context from the test context's components.
        let context = Context::new(
            ChainId::test(),
            test_ctx.db.clone(),
            test_ctx.mempool.ac_client.clone(),
            node_config.clone(),
            None,
        );

        let v2_config = V2Config::from_configs(&node_config.api_v2, &node_config.api);
        let v2_ctx = V2Context::new(context, v2_config);
        let router = build_v2_router(v2_ctx);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind failed");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        format!("http://{}", addr)
    })
}

fn bench_health(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_health", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/health", base_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_get_resources(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_get_resources_0x1", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/accounts/0x1/resources", base_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_get_single_resource(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_get_single_resource", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!(
                "{}/v2/accounts/0x1/resource/0x1::account::Account",
                base_url
            );
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_get_modules(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_get_modules_0x1", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/accounts/0x1/modules", base_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_get_single_module(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_get_single_module", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/accounts/0x1/module/account", base_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_list_transactions(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_list_transactions", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/transactions", base_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_get_latest_block(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_get_latest_block", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/blocks/latest", base_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_view_function(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    c.bench_function("v2_view_function", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/view", base_url);
            async move {
                let resp = client
                    .post(&url)
                    .json(&serde_json::json!({
                        "function": "0x1::account::exists_at",
                        "type_arguments": [],
                        "arguments": ["0x1"]
                    }))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(resp.status(), 200);
                let _body: serde_json::Value = resp.json().await.unwrap();
            }
        })
    });
}

fn bench_batch_request(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let base_url = setup_v2_server(&rt);
    let client = reqwest::Client::new();

    let mut group = c.benchmark_group("v2_batch");
    for batch_size in [1, 5, 10, 20] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &size| {
                let requests: Vec<serde_json::Value> = (0..size)
                    .map(|i| {
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "get_ledger_info",
                            "params": {},
                            "id": i
                        })
                    })
                    .collect();

                b.to_async(&rt).iter(|| {
                    let client = client.clone();
                    let url = format!("{}/v2/batch", base_url);
                    let reqs = requests.clone();
                    async move {
                        let resp = client.post(&url).json(&reqs).send().await.unwrap();
                        assert_eq!(resp.status(), 200);
                        let _body: serde_json::Value = resp.json().await.unwrap();
                    }
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    name = v2_benches;
    config = Criterion::default().sample_size(50);
    targets =
        bench_health,
        bench_get_resources,
        bench_get_single_resource,
        bench_get_modules,
        bench_get_single_module,
        bench_list_transactions,
        bench_get_latest_block,
        bench_view_function,
        bench_batch_request,
);
criterion_main!(v2_benches);
