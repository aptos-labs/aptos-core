// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Performance benchmarks for the v2 API.
//!
//! Measures request latency and throughput for key v2 endpoints, and
//! includes head-to-head comparisons against v1.
//! Run with: `cargo bench -p aptos-api`

use aptos_api::{
    attach_poem_to_runtime,
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

/// Spin up a v1 Poem server on a random port. Returns the base URL.
fn setup_v1_server(rt: &tokio::runtime::Runtime) -> String {
    rt.block_on(async {
        let mut node_config = NodeConfig::default();
        node_config.storage.rocksdb_configs.enable_storage_sharding = false;

        let test_ctx = new_test_context("bench_v1".to_string(), node_config.clone(), false);

        let context = Context::new(
            ChainId::test(),
            test_ctx.db.clone(),
            test_ctx.mempool.ac_client.clone(),
            node_config.clone(),
            None,
        );

        let poem_addr = attach_poem_to_runtime(
            &tokio::runtime::Handle::current(),
            context,
            &node_config,
            true, // random_port
            None,
        )
        .expect("Failed to start v1 Poem server");

        // Give Poem a moment to bind.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        format!("http://{}", poem_addr)
    })
}

/// Setup both servers and return (v2_url, v1_url).
fn setup_both_servers(rt: &tokio::runtime::Runtime) -> (String, String) {
    (setup_v2_server(rt), setup_v1_server(rt))
}

// ============================================================================
// v2-only benchmarks
// ============================================================================

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

// ============================================================================
// Head-to-head v1 vs v2 benchmarks
// ============================================================================

fn bench_v1_vs_v2_health(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let (v2_url, v1_url) = setup_both_servers(&rt);
    let client = reqwest::Client::new();

    let mut group = c.benchmark_group("health");

    group.bench_function("v1", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v1/-/healthy", v1_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.bench_function("v2", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/health", v2_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.finish();
}

fn bench_v1_vs_v2_ledger_info(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let (v2_url, v1_url) = setup_both_servers(&rt);
    let client = reqwest::Client::new();

    let mut group = c.benchmark_group("ledger_info");

    group.bench_function("v1", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v1", v1_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.bench_function("v2", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/info", v2_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.finish();
}

fn bench_v1_vs_v2_resources(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let (v2_url, v1_url) = setup_both_servers(&rt);
    let client = reqwest::Client::new();

    let mut group = c.benchmark_group("get_resources_0x1");

    group.bench_function("v1", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v1/accounts/0x1/resources", v1_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.bench_function("v2", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/accounts/0x1/resources", v2_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.finish();
}

fn bench_v1_vs_v2_single_resource(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let (v2_url, v1_url) = setup_both_servers(&rt);
    let client = reqwest::Client::new();

    let mut group = c.benchmark_group("get_single_resource");

    group.bench_function("v1", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!(
                "{}/v1/accounts/0x1/resource/0x1::account::Account",
                v1_url
            );
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.bench_function("v2", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!(
                "{}/v2/accounts/0x1/resource/0x1::account::Account",
                v2_url
            );
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.finish();
}

fn bench_v1_vs_v2_transactions(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    let (v2_url, v1_url) = setup_both_servers(&rt);
    let client = reqwest::Client::new();

    let mut group = c.benchmark_group("list_transactions");

    group.bench_function("v1", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v1/transactions", v1_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

    group.bench_function("v2", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = format!("{}/v2/transactions", v2_url);
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body = resp.bytes().await.unwrap();
            }
        })
    });

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

criterion_group!(
    name = v1_vs_v2_benches;
    config = Criterion::default().sample_size(30);
    targets =
        bench_v1_vs_v2_health,
        bench_v1_vs_v2_ledger_info,
        bench_v1_vs_v2_resources,
        bench_v1_vs_v2_single_resource,
        bench_v1_vs_v2_transactions,
);

criterion_main!(v2_benches, v1_vs_v2_benches);
