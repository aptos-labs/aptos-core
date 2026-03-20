# Aptos Telemetry Service Specification

## Overview

The Aptos Telemetry Service is a centralized service that collects telemetry data (metrics, logs, and custom events) from Aptos network nodes. It supports two authentication modes:

1. **Standard Node Authentication** - For validators, validator full nodes, and public full nodes in the Aptos network
2. **Custom Contract Authentication** - For third-party applications that maintain their own on-chain allowlists

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Aptos Telemetry Service                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  Standard    │  │   Custom     │  │   Standard   │  │   Custom     │    │
│  │  Auth        │  │   Contract   │  │   Ingest     │  │   Contract   │    │
│  │  (/auth)     │  │   Auth       │  │   Endpoints  │  │   Ingest     │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
│         │                 │                  │                 │            │
│         └─────────────────┴──────────────────┴─────────────────┘            │
│                                    │                                         │
│                              JWT Service                                     │
│                                    │                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                             Backend Sinks                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  Victoria    │  │    Humio     │  │    Loki      │  │   BigQuery   │    │
│  │  Metrics     │  │    (Logs)    │  │    (Logs)    │  │   (Events)   │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Part 1: Standard Node Authentication

### Node Types

| Node Type | Description |
|-----------|-------------|
| `Validator` | Active validators in the current epoch |
| `ValidatorFullNode` | Full nodes operated by validators |
| `PublicFullNode` | Full nodes in the configured allowlist |
| `UnknownValidator` | Self-identified validators not in the validator set |
| `UnknownFullNode` | Full nodes not in any allowlist |
| `Unknown` | Unclassified nodes |

### Authentication Flow

```
┌──────────┐                    ┌───────────────────┐
│  Aptos   │                    │    Telemetry      │
│  Node    │                    │    Service        │
└────┬─────┘                    └─────────┬─────────┘
     │                                    │
     │  1. GET /api/v1/                   │
     │───────────────────────────────────>│
     │                                    │
     │  { public_key: <server_pubkey> }   │
     │<───────────────────────────────────│
     │                                    │
     │  2. POST /api/v1/auth              │
     │  { chain_id, peer_id, role_type,   │
     │    server_public_key,              │
     │    handshake_msg (Noise IK) }      │
     │───────────────────────────────────>│
     │                                    │
     │  { handshake_msg (encrypted JWT) } │
     │<───────────────────────────────────│
     │                                    │
     │  3. POST /api/v1/ingest/metrics    │
     │  Authorization: Bearer <JWT>       │
     │───────────────────────────────────>│
     │                                    │
```

### Authentication Details

**Protocol**: Noise IK handshake with Ed25519/X25519 keys

**Prologue**: `chain_id (1 byte) | peer_id (32 bytes) | server_public_key (32 bytes)`

**Validation**:
1. Parse Noise handshake message with prologue
2. Extract client's public key from handshake
3. Look up peer in validator/VFN set for the chain
4. Verify public key matches registered keys
5. For unknown peers, verify peer_id is derived from public key
6. Issue JWT with appropriate `NodeType`

### JWT Claims

```json
{
  "chain_id": 1,
  "peer_id": "0x...",
  "node_type": "Validator",
  "epoch": 123,
  "run_uuid": "uuid-v4",
  "iat": 1234567890,
  "exp": 1234571490
}
```

### API Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/api/v1/` | GET | None | Get server public key |
| `/api/v1/health` | GET | None | Health check |
| `/api/v1/chain-access/{chain_id}` | GET | None | Check if chain is supported |
| `/api/v1/auth` | POST | Noise | Authenticate and get JWT |
| `/api/v1/ingest/metrics` | POST | JWT | Push Prometheus metrics |
| `/api/v1/ingest/logs` | POST | JWT | Push logs |
| `/api/v1/ingest/custom-event` | POST | JWT | Push custom events to BigQuery |
| `/api/v1/telemetry_log_env` | POST | JWT | Get log environment routing |

### Metrics Ingestion

**Endpoint**: `POST /api/v1/ingest/metrics`

**Headers**:
- `Authorization: Bearer <jwt>`
- `Content-Encoding: gzip` (optional)

**Body**: Prometheus exposition format (text or protobuf)

**Extra Labels Added**:
- `role` - Node type
- `chain_name` - Chain identifier
- `namespace` - `telemetry-service`
- `kubernetes_pod_name` - `peer_id:{identity}//{peer_id_hex}` or `peer_id:{peer_id_hex}`
- `run_uuid` - Session UUID
- `metrics_source` - `telemetry-service`

**Routing**:
- Known nodes (Validator, VFN, PFN) → `ingest_metrics_client`
- Unknown nodes → `untrusted_ingest_metrics_clients`

### Logs Ingestion

**Endpoint**: `POST /api/v1/ingest/logs`

**Headers**:
- `Authorization: Bearer <jwt>`
- `Content-Encoding: gzip` (optional)

**Body**: JSON array of log message strings

**Tags Added**:
- `chain_id` - Chain identifier
- `peer_role` - Node type
- `run_uuid` - Session UUID

**Fields Added**:
- `peer_id` - Node's peer ID
- `epoch` - Current epoch

**Routing**:
- Known nodes → `known_logs_ingest_client`
- Unknown nodes → `unknown_logs_ingest_client`

### Custom Events Ingestion

**Endpoint**: `POST /api/v1/ingest/custom-event`

**Body**:
```json
{
  "client_id": "string",
  "user_id": "peer_id_hex",
  "timestamp_micros": "1234567890000",
  "events": [
    {
      "name": "event_name",
      "params": { "key": "value" }
    }
  ]
}
```

**Destination**: Google BigQuery table

---

## Part 2: Custom Contract Authentication

Custom contracts allow third-party applications to use the telemetry service with their own on-chain allowlists.

### Node Types

| Node Type | Description |
|-----------|-------------|
| `Custom(contract_name)` | Trusted nodes - in on-chain allowlist OR static_allowlist |
| `CustomUnknown(contract_name)` | Authenticated but not in any allowlist (requires `allow_unknown_nodes: true`) |

### Trust Determination Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                        Authentication                             │
│                                                                   │
│  1. Verify Ed25519 signature (proves address ownership)           │
│                              │                                    │
│                              ▼                                    │
│  2. Check static_allowlist ──► In list? ──► YES ──► TRUSTED       │
│                              │                    (Custom)        │
│                              ▼ NO                                 │
│  3. Check on_chain_auth ─────► Configured? ─► YES ─► Check cache  │
│                              │                        │           │
│                              ▼ NO                     ▼           │
│  4. allow_unknown_nodes? ────► YES ──► UNTRUSTED    In list?      │
│                              │      (CustomUnknown)   │           │
│                              ▼ NO                     ▼           │
│                           REJECTED               YES: TRUSTED     │
│                                                  NO: goto step 4  │
└──────────────────────────────────────────────────────────────────┘
```

**Trust Sources** (in priority order):
1. `static_allowlist` - Config-based trust, no on-chain calls
2. `on_chain_auth` - Dynamic trust via on-chain allowlist verification
3. `allow_unknown_nodes` - Accept as untrusted (routes to untrusted sinks)

### Authentication Flow

```
┌──────────┐                    ┌───────────────────┐
│  Client  │                    │    Telemetry      │
│  Node    │                    │    Service        │
└────┬─────┘                    └─────────┬─────────┘
     │                                    │
     │  1. POST /api/v1/custom-contract   │
     │       /{name}/auth-challenge       │
     │  { address, chain_id }             │
     │───────────────────────────────────>│
     │                                    │
     │  { challenge: "uuid", expires_at } │
     │<───────────────────────────────────│
     │                                    │
     │  2. POST /api/v1/custom-contract   │
     │       /{name}/auth                 │
     │  { address, chain_id, challenge,   │
     │    signature, public_key }         │
     │───────────────────────────────────>│
     │                                    │
     │  { token: "<jwt>" }                │
     │<───────────────────────────────────│
     │                                    │
     │  3. POST /api/v1/custom-contract   │
     │       /{name}/ingest/metrics       │
     │  Authorization: Bearer <JWT>       │
     │───────────────────────────────────>│
     │                                    │
```

### Authentication Details

**Challenge-Response Protocol**:
1. Client requests challenge for their address
2. Service generates random UUID challenge, stores with expiration
3. Client signs challenge with Ed25519 private key
4. Service verifies:
   - Challenge exists and not expired (prevents replay)
   - Signature is valid
   - Public key derives to claimed address
   - Address is in on-chain allowlist (or `allow_unknown_nodes: true`)

**On-Chain Allowlist Verification Methods**:

| Method | Description |
|--------|-------------|
| `view_function` | Call a Move view function that returns address list |
| `resource` | Read a Move resource and extract address list from a field |

### Configuration

```yaml
custom_contract_configs:
  - name: "my_provider"

    # On-chain authentication (optional - omit for "open telemetry" mode)
    on_chain_auth:
      chain_id: 1
      rest_url: "https://api.mainnet.aptoslabs.com/v1"

      # Method 1: View function
      method: view_function
      resource_path: "0x123::module::get_members"
      function_args: []              # Optional arguments
      address_list_field: "[0].addr" # JSON path to extract addresses

      # Method 2: Resource read (alternative)
      # method: resource
      # resource_path: "0x123::module::MemberRegistry"
      # address_list_field: "members"

    # Static allowlist (optional) - trusted addresses via config
    # Addresses here are treated as "trusted" without on-chain verification.
    # Useful for RPCs where you know operators but don't want on-chain overhead.
    # Trust priority: 1) static_allowlist, 2) on_chain_auth, 3) allow_unknown_nodes
    static_allowlist:
      1:  # Chain ID
        - "0xabc123..."  # Known operator 1
        - "0xdef456..."  # Known operator 2

    # Custom node type name for metrics labels
    node_type_name: "my_provider_node"

    # Allow nodes not in allowlist (routes to untrusted sinks)
    allow_unknown_nodes: true

    # Trusted node sinks (for allowlisted nodes - both on_chain and static_allowlist)
    metrics_sinks:
      - type: victoria_metrics
        endpoint_url: "https://vm.example.com/api/v1/import/prometheus"
        key_env_var: "VM_TOKEN"

    logs_sink:
      type: humio
      endpoint_url: "https://cloud.humio.com/"
      key_env_var: "HUMIO_TOKEN"

    events_sink:
      type: bigquery
      project_id: "my-project"
      dataset_id: "telemetry"
      table_id: "events"

    # Untrusted node sinks (for non-allowlisted nodes)
    untrusted_metrics_sinks:
      - type: victoria_metrics
        endpoint_url: "https://vm.example.com/api/v1/import/prometheus"
        key_env_var: "VM_UNTRUSTED_TOKEN"

    untrusted_logs_sink:
      type: humio
      endpoint_url: "https://cloud.humio.com/"
      key_env_var: "HUMIO_UNTRUSTED_TOKEN"

    # Rate limiting for untrusted nodes
    untrusted_metrics_rate_limit:
      requests_per_second: 10
      burst_capacity: 20
      enabled: true

    untrusted_logs_rate_limit:
      requests_per_second: 5
      burst_capacity: 10
      enabled: true

    # Per-peer configuration
    peer_identities:
      1:  # Chain ID
        "0xabc...": "node-1"
        "0xdef...": "node-2"

    blacklist_peers:
      - "0xbad..."
```

### API Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/api/v1/custom-contract/{name}/auth-challenge` | POST | None | Get authentication challenge |
| `/api/v1/custom-contract/{name}/auth` | POST | None | Exchange signed challenge for JWT |
| `/api/v1/custom-contract/{name}/ingest/metrics` | POST | JWT | Push Prometheus metrics |
| `/api/v1/custom-contract/{name}/ingest/logs` | POST | JWT | Push logs |
| `/api/v1/custom-contract/{name}/ingest/custom-event` | POST | JWT | Push custom events |

### Metrics Ingestion

**Extra Labels Added**:
- `peer_id` - Client's address
- `node_type` - Custom node type name
- `contract_name` - Contract identifier
- `trust_status` - `trusted` or `untrusted`
- `kubernetes_pod_name` - `peer_id:{identity}//{peer_id_hex}` (if identity configured)

### Security Features

**Cross-Contract Token Reuse Prevention**:
- Contract name is embedded in JWT's `NodeType`
- Each ingest endpoint verifies JWT contract name matches URL path
- Prevents using token from contract_A to inject data into contract_B

**Replay Attack Prevention**:
- Challenges are single-use and expire after 5 minutes
- Challenge is consumed before signature verification

**Blacklist Support**:
- Per-contract `blacklist_peers` configuration
- Blacklisted peers receive 403 Forbidden

---

## Part 3: Rate Limiting

### Global Rate Limits (Standard Nodes)

```yaml
unknown_metrics_rate_limit:
  requests_per_second: 100
  burst_capacity: 200
  enabled: true

unknown_logs_rate_limit:
  requests_per_second: 100
  burst_capacity: 200
  enabled: true
```

### Per-Contract Rate Limits

Custom contracts can override global limits with their own configuration. The hierarchy is:

1. Per-contract rate limit (if configured) → Apply
2. No per-contract limit → Fall back to global limit

### Token Bucket Algorithm

- Tokens refill at `requests_per_second` rate
- Maximum tokens = `burst_capacity`
- Each request consumes 1 token
- Request rejected with 429 if no tokens available

---

## Part 4: Backend Sinks

### Victoria Metrics (Metrics)

- **Protocol**: Prometheus remote write
- **Endpoint**: `/api/v1/import/prometheus`
- **Authentication**: Bearer token or basic auth
- **Format**: Prometheus text format with extra labels as query params

### Humio (Logs)

- **Protocol**: Humio unstructured log ingest
- **Endpoint**: `/api/v1/ingest/humio-unstructured`
- **Authentication**: Bearer token or basic auth
- **Format**: JSON array of `UnstructuredLog` objects

### Loki (Logs)

- **Protocol**: Loki push API
- **Endpoint**: `/loki/api/v1/push`
- **Authentication**: Tenant ID header
- **Format**: Loki push request format

### BigQuery (Events)

- **Protocol**: BigQuery insertAll API
- **Authentication**: Service account
- **Format**: `BigQueryRow` with event identity and params

---

## Part 5: Caching

### Validator Set Cache

- Updated by `PeerSetCacheUpdater` background task
- Polls trusted full nodes for validator/VFN sets
- Used for standard node authentication

### Allowlist Cache

- Updated by `AllowlistCacheUpdater` background task
- Polls on-chain resources/view functions
- Configurable TTL (`allowlist_cache_ttl_secs`)
- Used for custom contract authentication

### Challenge Cache

- In-memory cache for authentication challenges
- TTL: 5 minutes
- Max 10 concurrent challenges per address
- Cleaned up periodically

---

## Part 6: Error Codes

| HTTP Status | Error Type | Description |
|-------------|------------|-------------|
| 400 | Bad Request | Invalid payload, signature, or challenge |
| 401 | Unauthorized | Missing or invalid JWT, validator set unavailable |
| 403 | Forbidden | Not in allowlist, blacklisted, or public key mismatch |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Backend failure |
| 503 | Service Unavailable | Sink not configured |

---

## Part 7: Observability

### Metrics Exported

- `telemetry_service_error_counts` - Errors by type
- `telemetry_service_metrics_ingest_backend_request_duration` - Metrics sink latency
- `telemetry_service_log_ingest_backend_request_duration` - Logs sink latency
- `telemetry_service_bigquery_backend_request_duration` - BigQuery latency
- `telemetry_service_custom_contract_errors` - Custom contract errors by type/endpoint

### Logging

- Structured JSON logging
- GCP Cloud Trace integration via `X-Cloud-Trace-Context` header
- Debug-level logging for request handling

---

## Appendix A: Configuration Reference

See `TelemetryServiceConfig` in `lib.rs` for full configuration schema.

## Appendix B: Example Configurations

### Standard Node Configuration

```yaml
address: "0.0.0.0:443"
tls_cert_path: "/certs/cert.pem"
tls_key_path: "/certs/key.pem"

trusted_full_node_addresses:
  1: "https://api.mainnet.aptoslabs.com/v1"
  2: "https://api.testnet.aptoslabs.com/v1"

metrics_endpoints_config:
  ingest_metrics_endpoint:
    endpoint_url: "https://vm.example.com/api/v1/import/prometheus"
    key_env_var: "VM_TOKEN"
  untrusted_metrics_endpoint:
    endpoint_url: "https://vm-untrusted.example.com/api/v1/import/prometheus"
    key_env_var: "VM_UNTRUSTED_TOKEN"

humio_ingest_config:
  known_logs_endpoint:
    endpoint_url: "https://cloud.humio.com/"
    key_env_var: "HUMIO_KNOWN_TOKEN"
  unknown_logs_endpoint:
    endpoint_url: "https://cloud.humio.com/"
    key_env_var: "HUMIO_UNKNOWN_TOKEN"

unknown_metrics_rate_limit:
  requests_per_second: 100
  burst_capacity: 200
  enabled: true
```

### Custom Contract Only Configuration

```yaml
address: "0.0.0.0:443"

# No standard node configuration - custom contracts only
custom_contract_configs:
  - name: "storage_providers"
    on_chain_auth:
      chain_id: 1
      method: view_function
      resource_path: "0x123::registry::get_providers"
      address_list_field: "[0].address"
    allow_unknown_nodes: true
    metrics_sinks:
      - type: victoria_metrics
        endpoint_url: "https://vm.example.com/api/v1/import/prometheus"
        key_env_var: "VM_STORAGE_TOKEN"
```
