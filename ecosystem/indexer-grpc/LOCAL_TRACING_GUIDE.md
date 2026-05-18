# End-to-End Distributed Tracing: Local Development Guide

This guide covers running the full gRPC indexer stack locally with Jaeger for
distributed trace visualization. All steps have been validated end-to-end.

## Architecture Overview

```
Client (grpcurl)
    │
    ▼
┌──────────────┐
│ API Gateway   │  ← OTLP traces to Jaeger
│ (port 50053)  │
└──────┬───────┘
       │ HTTP/2 proxy
       ▼
┌──────────────────┐
│ gRPC Gateway     │  ← OTLP traces to Jaeger
│ (port 50051)     │
└──────┬───────────┘
       │
       │  1. GetDataServiceForRequest  ──▶  gRPC Manager (50052)
       │     (asks: "which Data Service     returns Data Service address
       │      should handle this?")
       │
       │  2. Proxies HTTP/2 directly   ──▶  gRPC Data Service v2 (50054)
       │     to the resolved address
       ▼
┌─────────────────────────┐
│ gRPC Data Service v2    │  ← OTLP traces to Jaeger
│ (port 50054)            │
│                         │
│  ┌───────────────────┐  │        ┌──────────────────┐
│  │ Live path:        │──│──────▶ │ gRPC Manager     │
│  │ Fetches from      │  │  gRPC  │ (port 50052)     │
│  │ Manager's cache   │  │        │ OTLP traces ──▶  │
│  └───────────────────┘  │        │                  │
│                         │        │ Fetches from     │
│  ┌───────────────────┐  │        │ fullnode, caches  │
│  │ Historical path:  │  │        │ in memory, and   │
│  │ Reads from the    │  │        │ writes to the    │
│  │ local filestore   │  │        │ local filestore  │
│  └───────────────────┘  │        └──────┬───────────┘
│                         │               │
└─────────────────────────┘               │ FullnodeData/
                                          │ GetTransactionsFromNode
                                          ▼
                                   ┌──────────────────┐
                                   │ Localnet Node    │
                                   │ (port 50055)     │
                                   └──────────────────┘
```

### Key relationships

- The **gRPC Manager** serves two roles:
  1. **Discovery**: The gRPC Gateway asks the Manager which Data Service
     instance should handle a request. The Manager returns an address, and
     the Gateway proxies directly to that Data Service — the Manager is not
     in the data path.
  2. **Data ingestion**: The Manager connects to the fullnode via the
     `FullnodeData/GetTransactionsFromNode` gRPC interface, caches
     transactions in memory, and writes them to the local filestore.

- The **gRPC Data Service v2** has two data paths:
  1. **Live path**: For recent transactions. The Data Service's live
     sub-service fetches data from the **gRPC Manager's in-memory cache**
     (via `GrpcManager/GetTransactions`). This is the primary path and
     serves transactions that are still in the Manager's cache.
  2. **Historical path**: For older transactions. The historical sub-service
     reads from the **local filestore** (populated by the Manager's
     `FileStoreUploader`). The filestore accumulates transactions in files
     within folders of 100K transactions each.

- All services export OpenTelemetry traces to Jaeger when
  `OTEL_EXPORTER_OTLP_ENDPOINT` is set. The `indexer-grpc-server-framework`
  shared crate automatically adds an OTLP exporter layer alongside the JSON
  log formatter. Traces propagate across service boundaries via W3C
  `traceparent`/`tracestate` gRPC metadata, producing a single cross-service
  trace waterfall in Jaeger.

## Prerequisites

- Docker (for Jaeger, Postgres, Redis)
- Rust toolchain (see `scripts/dev_setup.sh`)
- The `api-gateway` repo checked out at a sibling path (e.g. `../api-gateway`)
- `grpcurl` (`brew install grpcurl`)

## Step 1: Start Jaeger

```bash
cd /path/to/api-gateway
./scripts/start-jaeger.sh
```

This starts Jaeger all-in-one in Docker with:
- **OTLP gRPC receiver**: `localhost:4317`
- **OTLP HTTP receiver**: `localhost:4318`
- **Jaeger UI**: http://localhost:16686

## Step 2: Start a Local Fullnode

The gRPC Manager expects the **FullnodeData** gRPC interface
(`GetTransactionsFromNode`), but the localnet's default txn stream uses the
**RawData** interface (`GetTransactions`). Use the
`--use-internal-fullnode-data-interface` flag to expose the correct interface:

```bash
cd /path/to/core
cargo run -p aptos -- node run-localnet \
  --use-internal-fullnode-data-interface \
  --txn-stream-port 50055 \
  --force-restart --assume-yes
```

Wait for the output:
```
Setup is complete, you can now use the localnet!
```

Verify the txn stream is serving:
```bash
lsof -i :50055  # Should show aptos LISTEN
```

## Step 3: Start the gRPC Manager

Config file at `ecosystem/indexer-grpc/configs/manager.yaml`:

```yaml
health_check_port: 8091
server_config:
  chain_id: 4
  service_config:
    listen_address: 0.0.0.0:50052
  cache_config:
    max_cache_size: 1073741824
    target_cache_size: 536870912
  file_store_config:
    file_store_type: LocalFileStore
    local_file_store_path: /tmp/indexer_grpc_filestore
  self_advertised_address: "http://127.0.0.1:50052"
  grpc_manager_addresses:
    - "http://127.0.0.1:50052"
  fullnode_addresses:
    - "http://127.0.0.1:50055"
  is_master: true
  allow_fn_fallback: true
```

Start:

```bash
cd /path/to/core
rm -rf /tmp/indexer_grpc_filestore && mkdir -p /tmp/indexer_grpc_filestore
kill $(lsof -t -i:50052); kill $(lsof -t -i:8091)
OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 \
OTEL_SERVICE_NAME=grpc-manager \
OTEL_TRACES_SAMPLER=always_on \
RUST_LOG=info \
  cargo run -p aptos-indexer-grpc-manager -- \
  --config-path ecosystem/indexer-grpc/configs/manager.yaml
```

Look for: `MetadataManager is created ... fullnode_addresses: ["http://127.0.0.1:50055"]`

**Important**: All addresses in the config must include the `http://` scheme
prefix. The health check port (8091) must not collide with the localnet faucet
(8081).

## Step 4: Start the gRPC Data Service v2

Config file at `ecosystem/indexer-grpc/configs/data-service-v2.yaml`:

```yaml
health_check_port: 8083
server_config:
  chain_id: 4
  service_config:
    listen_address: 0.0.0.0:50054
    tls_config: null
  live_data_service_config:
    enabled: true
    num_slots: 100000
    size_limit_bytes: 1073741824
  historical_data_service_config:
    enabled: true
    file_store_config:
      file_store_type: LocalFileStore
      local_file_store_path: /tmp/indexer_grpc_filestore
  grpc_manager_addresses:
    - "http://127.0.0.1:50052"
  self_advertised_address: "http://127.0.0.1:50054"
```

Start:

```bash
cd /path/to/core
kill $(lsof -t -i:50054); kill $(lsof -t -i:8083)
OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 \
OTEL_SERVICE_NAME=grpc-data-service \
OTEL_TRACES_SAMPLER=always_on \
RUST_LOG=info \
  cargo run -p aptos-indexer-grpc-data-service-v2 -- \
  --config-path ecosystem/indexer-grpc/configs/data-service-v2.yaml
```

The Data Service will heartbeat the Manager. Wait until you see:
```
Received known_latest_version (XX) from GrpcManager http://127.0.0.1:50052
```

**Note**: The Data Service blocks during startup until it receives a non-zero
`known_latest_version` from the Manager. If the localnet isn't producing
transactions yet, the Data Service will wait. This is normal.

## Step 5: Start the gRPC Gateway

Config file at `ecosystem/indexer-grpc/configs/gateway.yaml`:

```yaml
health_check_port: 8085
server_config:
  port: 50051
  grpc_manager_address: "http://127.0.0.1:50052"
```

Start:

```bash
cd /path/to/core
kill $(lsof -t -i:50051); kill $(lsof -t -i:8085)
OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 \
OTEL_SERVICE_NAME=grpc-gateway \
OTEL_TRACES_SAMPLER=always_on \
RUST_LOG=info \
  cargo run -p aptos-indexer-grpc-gateway -- \
  --config-path ecosystem/indexer-grpc/configs/gateway.yaml
```

Look for: `gRPC Gateway listening on 0.0.0.0:50051`

## Step 6: Start the API Gateway

First, start the backing stores (Postgres, Redis):

```bash
cd /path/to/api-gateway
./scripts/run-stores.sh --no-pubsub
```

Use the local tracing config at `configs/config.local-tracing.yaml` (see
api-gateway repo for the full config). Key settings:

```yaml
role: grpc-gateway
grpc_proxy_config:
  upstream_url: http://127.0.0.1:50051
  data_service_authorization_key: "test-key"
  listen_port: 50053
  fail_open: true
redis_config:
  redis_url: redis://127.0.0.1:6379
  redis_cluster: false
database_url: "postgresql://postgres:postgres@127.0.0.1:5434/postgres?schema=public"
```

Seed a test API key in Redis so requests can authenticate:

```bash
redis-cli SET 'local:cache-worker:api-key-secret:test123' \
  '{"api_key_name":"test-key","application_id":"test-app-id","allowed_networks":["local"],"service_type":"Api","per_ip_limit_rules":[],"per_ip_stream_limit":null,"org_level_active_stream_limit":100,"http_limit_rules":[],"traffic_tier":"standard","application_name":"test-app","web_app_urls":[],"extension_ids":[],"enforce_origin":false,"organization_id":"test-org","project_id":"test-project","organization_name":"test-org-name","project_name":"test-project-name","usage_blocked_reason":null}'
```

Start with OTLP export enabled:

```bash
cd /path/to/api-gateway
kill $(lsof -t -i:50053)
OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 \
OTEL_SERVICE_NAME=api-gateway \
OTEL_TRACES_SAMPLER=always_on \
DATABASE_URL="postgresql://postgres:postgres@127.0.0.1:5434/postgres?schema=public" \
RUST_LOG=info \
  cargo run -- --config-path configs/config.local-tracing.yaml
```

Look for: `GRPC Gateway listening on 0.0.0.0:50053`

## Step 7: Test the Live Data Path

The live path serves recent transactions from the gRPC Manager's in-memory
cache. This is the path you'll exercise in the local setup, since the cache
holds all transactions produced by the localnet.

Request recent transactions through the full stack:

```bash
cd /path/to/core
grpcurl \
  -max-msg-sz 10000000 \
  -d '{ "starting_version": 0, "transactions_count": 5 }' \
  -import-path protos/proto \
  -proto aptos/indexer/v1/raw_data.proto \
  -plaintext \
  -H 'authorization: Bearer test123' \
  127.0.0.1:50053 \
  aptos.indexer.v1.RawData/GetTransactions
```

You should see transaction data returned as JSON. Check the Data Service logs
for confirmation the live path was used:

```
"Dispatching get_transactions request", service_type: "live"
```

## Step 8 (Optional): Test the Historical Data Path

The historical path reads from the local filestore instead of the Manager's
cache. In production this activates when a client requests a version that has
been evicted from the live cache but is available in the filestore. The
`DataServiceWrapperWrapper` first tries the live path (peeks the stream); if no
data is returned, it falls back to the historical path.

By default the `FileStoreUploader` writes files in 100K-transaction folders and
50 MB batches — far too large for a localnet. To test the historical path
locally you need two adjustments:

### 1. Shrink the filestore batch constants

Temporarily edit
`ecosystem/indexer-grpc/indexer-grpc-manager/src/file_store_uploader.rs`:

```rust
// Before (production values):
const NUM_TXNS_PER_FOLDER: u64 = 100000;
const MAX_SIZE_PER_FILE: usize = 50 * (1 << 20);

// After (local testing):
const NUM_TXNS_PER_FOLDER: u64 = 100;
const MAX_SIZE_PER_FILE: usize = 1024;
```

Wipe the filestore so the metadata matches the new folder size:

```bash
rm -rf /tmp/indexer_grpc_filestore && mkdir -p /tmp/indexer_grpc_filestore
```

Restart the gRPC Manager (Step 3). You should see it writing files:
```
Dumping transactions [100, 101] to file "1/100".
```

### 2. Disable the live sub-service

When both services are enabled, the wrapper always tries live first and succeeds
(since the localnet's ~900 transactions all fit in the Manager's cache). To
force requests through the historical path, temporarily disable live in the Data
Service config (`ecosystem/indexer-grpc/configs/data-service-v2.yaml`):

```yaml
  live_data_service_config:
    enabled: false        # temporarily disabled
```

Restart the Data Service (Step 4). Now send a request:

```bash
grpcurl \
  -max-msg-sz 10000000 \
  -d '{ "starting_version": 50, "transactions_count": 3 }' \
  -import-path protos/proto \
  -proto aptos/indexer/v1/raw_data.proto \
  -plaintext \
  -H 'authorization: Bearer test123' \
  127.0.0.1:50053 \
  aptos.indexer.v1.RawData/GetTransactions
```

Check the Data Service logs for:
```
"Dispatching get_transactions request", service_type: "historical"
```

**Remember to revert both changes** (restore the constants and re-enable live)
after testing.

## Step 9: View Traces in Jaeger

Open http://localhost:16686 in your browser.

1. Select any service (e.g. **api-gateway**, **grpc-gateway**,
   **grpc-data-service**, **grpc-manager**) from the "Service" dropdown.
2. Click **Find Traces**.
3. Click on a trace to see the span waterfall.

A request through the full stack produces a **cross-service trace** that shows
spans from multiple services in a single waterfall:

- `api-gateway: POST /{*path}` — incoming gRPC request
- `api-gateway: call_grpc_data_service` — proxy to the gRPC Gateway
- `grpc-gateway: grpc_gateway.get_data_service_url` — service discovery + proxy
- `grpc-data-service: data_service.get_transactions` — request dispatch
- `grpc-data-service: data_service_wrapper.get_transactions` — live or historical

The services also write structured JSON logs with `trace_id` and
`parent_span_id` fields that can be correlated with the Jaeger UI.

## Port Reference

| Service              | gRPC Port | Health Port | Notes                                    |
|----------------------|-----------|-------------|------------------------------------------|
| Localnet Node        | 50055     | -           | FullnodeData interface                   |
| gRPC Manager         | 50052     | 8091        | Discovery + data ingestion from fullnode |
| gRPC Data Service v2 | 50054     | 8083        | Live + historical data                   |
| gRPC Gateway         | 50051     | 8085        | Discovery via Manager, proxy to Data Svc |
| API Gateway (gRPC)   | 50053     | 8711        | Auth + proxy to gRPC Gateway             |
| Jaeger UI            | -         | 16686       | Browser UI                               |
| Jaeger OTLP gRPC     | 4317      | -           | Trace receiver                           |
| Postgres             | 5434      | -           | For API Gateway                          |
| Redis                | 6379      | -           | For API Gateway                          |
