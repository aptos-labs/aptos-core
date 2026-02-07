# V2 API Design Specification

## Overview

This document provides a comprehensive specification for the Aptos REST API v2. The v2 API is designed with performance, developer experience, and future extensibility as primary goals.

**Design Status**: Proposal  
**Target Timeline**: 3-6 months for core implementation  
**Backward Compatibility**: v1 API will continue to be supported

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Response Format](#2-response-format)
3. [Error Handling](#3-error-handling)
4. [Endpoint Specification](#4-endpoint-specification)
5. [Type Definitions](#5-type-definitions)
6. [Query Parameters](#6-query-parameters)
7. [Batch Operations](#7-batch-operations)
8. [Streaming API](#8-streaming-api)
9. [Authentication & Rate Limiting](#9-authentication--rate-limiting)
10. [Migration Guide](#10-migration-guide)

---

## 1. Design Principles

### 1.1 Core Principles

| Principle | Description | v1 Comparison |
|-----------|-------------|---------------|
| **BCS-First** | Default to BCS encoding; JSON opt-in | v1 defaults to JSON |
| **Explicit** | No magic type resolution unless requested | v1 always resolves types |
| **Efficient** | Minimize response size and processing | v1 includes all fields |
| **Consistent** | Uniform response structure across all endpoints | v1 varies by endpoint |
| **Versioned** | All types include version for evolution | v1 types are unversioned |
| **Composable** | Support batch requests and field selection | v1 is single-request only |

### 1.2 URL Structure

```
/v2/
├── accounts/           # Account-related endpoints
├── transactions/       # Transaction endpoints
├── blocks/            # Block endpoints
├── events/            # Event endpoints
├── state/             # State query endpoints
├── view/              # View function execution
├── gas/               # Gas estimation
└── health/            # Health checks
```

### 1.3 HTTP Methods

| Method | Usage |
|--------|-------|
| `GET` | Read operations |
| `POST` | Write operations, complex queries |
| `HEAD` | Existence checks (new) |

---

## 2. Response Format

### 2.1 Success Response Envelope

All successful responses wrapped in a standard envelope:

```typescript
interface ApiResponse<T> {
  // The response data
  data: T;
  
  // Ledger state at time of response
  ledger_info: LedgerInfoSummary;
  
  // Pagination cursor (if applicable)
  cursor?: string;
  
  // Non-fatal warnings
  warnings?: ApiWarning[];
}

interface LedgerInfoSummary {
  chain_id: number;
  ledger_version: string;       // u64 as string
  ledger_timestamp_usec: string; // u64 as string
  oldest_ledger_version: string;
  block_height: string;
}

interface ApiWarning {
  code: string;
  message: string;
}
```

### 2.2 Response Headers

| Header | Description |
|--------|-------------|
| `X-Aptos-Chain-Id` | Chain identifier |
| `X-Aptos-Ledger-Version` | Current ledger version |
| `X-Aptos-Ledger-Timestamp` | Current ledger timestamp (usec) |
| `X-Aptos-Cursor` | Pagination cursor (if applicable) |
| `X-Aptos-Request-Id` | Unique request identifier for tracing |
| `Content-Type` | `application/json` or `application/x-bcs` |

### 2.3 Content Negotiation

**Request Headers:**

| Accept Header | Response Format |
|---------------|-----------------|
| `application/x-bcs` (default) | BCS-encoded response |
| `application/json` | JSON response |
| `application/x-ndjson` | Newline-delimited JSON (streaming) |

**Response Content-Type:**

```
Content-Type: application/json; charset=utf-8
Content-Type: application/x-bcs
Content-Type: application/x-ndjson
```

---

## 3. Error Handling

### 3.1 Error Response Format

```typescript
interface ApiError {
  // Machine-readable error code
  error_code: ErrorCode;
  
  // Human-readable message
  message: string;
  
  // Ledger info (if available)
  ledger_info?: LedgerInfoSummary;
  
  // VM error details (if applicable)
  vm_error?: VmErrorDetails;
  
  // Request ID for support
  request_id: string;
  
  // Detailed error info (debug mode only)
  details?: Record<string, any>;
}

interface VmErrorDetails {
  // VM status code
  status_code: number;
  
  // Abort location (if move abort)
  location?: string;
  
  // Abort code (if move abort)
  abort_code?: string;
  
  // Human-readable explanation
  explanation: string;
}

// Error codes enum
type ErrorCode =
  | "INVALID_INPUT"
  | "RESOURCE_NOT_FOUND"
  | "ACCOUNT_NOT_FOUND"
  | "TRANSACTION_NOT_FOUND"
  | "BLOCK_NOT_FOUND"
  | "VERSION_NOT_FOUND"
  | "VERSION_PRUNED"
  | "VM_ERROR"
  | "MEMPOOL_FULL"
  | "SEQUENCE_NUMBER_TOO_OLD"
  | "RATE_LIMITED"
  | "INTERNAL_ERROR"
  | "SERVICE_UNAVAILABLE";
```

### 3.2 HTTP Status Codes

| Status | Error Code | Description |
|--------|------------|-------------|
| 400 | `INVALID_INPUT` | Bad request / validation error |
| 404 | `*_NOT_FOUND` | Resource not found |
| 410 | `VERSION_PRUNED` | Data has been pruned |
| 422 | `VM_ERROR` | Transaction failed VM validation |
| 429 | `RATE_LIMITED` | Rate limit exceeded |
| 500 | `INTERNAL_ERROR` | Server error |
| 503 | `SERVICE_UNAVAILABLE` | Service temporarily unavailable |
| 507 | `MEMPOOL_FULL` | Mempool is full |

---

## 4. Endpoint Specification

### 4.1 Accounts

#### Get Account

```
GET /v2/accounts/{address}
```

**Parameters:**
- `address` (path): Account address
- `ledger_version` (query, optional): Historical version

**Response:**
```typescript
interface AccountDataV2 {
  address: string;
  sequence_number: string;
  authentication_key: string;
}
```

#### Get Account Resources

```
GET /v2/accounts/{address}/resources
```

**Parameters:**
- `address` (path): Account address
- `ledger_version` (query, optional): Historical version
- `start` (query, optional): Pagination cursor
- `limit` (query, optional): Page size (default: 25, max: 1000)
- `include` (query, optional): Additional fields to include

**Response:**
```typescript
interface MoveResourceV2 {
  type: string;              // Full struct tag
  data: any;                 // Resource data (if JSON)
  state_key_hash?: string;   // Include with include=state_key_hash
  size_bytes?: number;       // Include with include=size
}
```

#### Get Account Balance

```
GET /v2/accounts/{address}/balance/{asset_type}
```

**Parameters:**
- `address` (path): Account address
- `asset_type` (path): Coin type or FA metadata address
- `ledger_version` (query, optional): Historical version

**Response:**
```typescript
interface BalanceV2 {
  amount: string;            // Balance amount as string
  asset_type: string;        // Normalized asset identifier
  decimals?: number;         // Asset decimals (if known)
}
```

### 4.2 Transactions

#### List Transactions

```
GET /v2/transactions
```

**Parameters:**
- `start` (query, optional): Starting version or cursor
- `limit` (query, optional): Page size
- `include` (query, optional): `events`, `changes`, `payload`

**Response:** `ApiResponse<TransactionV2[]>`

#### Get Transaction by Hash

```
GET /v2/transactions/by_hash/{hash}
```

**Parameters:**
- `hash` (path): Transaction hash
- `include` (query, optional): `events`, `changes`, `payload`

#### Get Transaction by Version

```
GET /v2/transactions/by_version/{version}
```

#### Submit Transaction

```
POST /v2/transactions/submit
```

**Request Body:**
```typescript
// BCS-encoded SignedTransaction (preferred)
Content-Type: application/x-bcs

// Or JSON
Content-Type: application/json
{
  "sender": "0x...",
  "sequence_number": "0",
  "payload": { ... },
  "max_gas_amount": "10000",
  "gas_unit_price": "100",
  "expiration_timestamp_secs": "1234567890",
  "signature": { ... }
}
```

**Response:**
```typescript
interface SubmitResultV2 {
  hash: string;
  // Only included if wait=true
  transaction?: TransactionV2;
}
```

#### Simulate Transaction

```
POST /v2/transactions/simulate
```

**Query Parameters:**
- `estimate_gas` (boolean): Estimate max gas
- `estimate_gas_price` (boolean): Use estimated gas price

#### Submit Batch

```
POST /v2/transactions/submit_batch
```

**Request Body:**
```typescript
{
  "transactions": SignedTransaction[]  // BCS or JSON
}
```

**Response:**
```typescript
interface BatchSubmitResultV2 {
  results: Array<{
    index: number;
    hash?: string;
    error?: ApiError;
  }>;
  successful_count: number;
  failed_count: number;
}
```

### 4.3 Blocks

#### Get Latest Block

```
GET /v2/blocks/latest
```

**Parameters:**
- `include` (query, optional): `transactions`

#### Get Block by Height

```
GET /v2/blocks/by_height/{height}
```

#### Get Block by Version

```
GET /v2/blocks/by_version/{version}
```

### 4.4 Events

#### Get Events by Key

```
GET /v2/events/by_key/{address}/{creation_number}
```

**Parameters:**
- `start` (query, optional): Starting sequence number
- `limit` (query, optional): Page size

### 4.5 State

#### Get Resource

```
GET /v2/state/resource/{address}/{resource_type}
```

**Parameters:**
- `ledger_version` (query, optional)
- `decode` (query, optional): Decode the resource (default: true for JSON)

#### Get Module

```
GET /v2/state/module/{address}/{module_name}
```

#### Get Table Item

```
POST /v2/state/table/{table_handle}
```

**Request Body:**
```typescript
{
  "key": any,
  "key_type": string,    // Optional for raw endpoint
  "value_type": string   // Optional for raw endpoint
}
```

### 4.6 View Functions

#### Execute View Function

```
POST /v2/view
```

**Request Body:**
```typescript
{
  "function": "0x1::coin::balance",
  "type_arguments": ["0x1::aptos_coin::AptosCoin"],
  "arguments": ["0x1"]
}
```

**Response:**
```typescript
interface ViewResultV2 {
  values: any[];
  gas_used: string;
}
```

#### Batch View Functions

```
POST /v2/view/batch
```

**Request Body:**
```typescript
{
  "requests": Array<{
    "id": string;        // Client-provided correlation ID
    "function": string;
    "type_arguments": string[];
    "arguments": any[];
  }>
}
```

**Response:**
```typescript
interface BatchViewResultV2 {
  results: Array<{
    id: string;
    values?: any[];
    gas_used?: string;
    error?: ApiError;
  }>;
}
```

### 4.7 Gas

#### Estimate Gas Price

```
GET /v2/gas/estimate
```

**Response:**
```typescript
interface GasEstimateV2 {
  gas_estimate: string;
  prioritized_gas_estimate: string;
  deprioritized_gas_estimate: string;
}
```

### 4.8 Health

#### Readiness Probe

```
GET /v2/health/ready
```

#### Liveness Probe

```
GET /v2/health/live
```

---

## 5. Type Definitions

### 5.1 Transaction Types

```typescript
interface TransactionV2 {
  version: string;
  hash: string;
  state_change_hash: string;
  event_root_hash: string;
  gas_used: string;
  success: boolean;
  vm_status: string;
  accumulator_root_hash: string;
  timestamp_usec: string;
  
  // Type-specific fields
  type: TransactionType;
  
  // For user transactions
  sender?: string;
  sequence_number?: string;
  max_gas_amount?: string;
  gas_unit_price?: string;
  expiration_timestamp_secs?: string;
  payload?: TransactionPayloadV2;
  signature?: TransactionSignatureV2;
  
  // Optional includes
  events?: EventV2[];
  changes?: WriteSetChangeV2[];
}

type TransactionType =
  | "user_transaction"
  | "block_metadata"
  | "state_checkpoint"
  | "genesis"
  | "validator";

interface TransactionPayloadV2 {
  type: PayloadType;
  
  // Entry function
  function?: string;           // "0x1::coin::transfer"
  type_arguments?: string[];
  arguments?: any[];
  
  // Script
  code?: string;               // Hex-encoded bytecode
  
  // Multisig
  multisig_address?: string;
}

type PayloadType = "entry_function" | "script" | "multisig";
```

### 5.2 Event Types

```typescript
interface EventV2 {
  guid: {
    creation_number: string;
    account_address: string;
  };
  sequence_number: string;
  type: string;
  data: any;
}

interface VersionedEventV2 extends EventV2 {
  version: string;
}
```

### 5.3 Write Set Changes

```typescript
interface WriteSetChangeV2 {
  type: ChangeType;
  state_key_hash: string;
  
  // Resource changes
  address?: string;
  resource_type?: string;
  data?: any;
  
  // Module changes
  module?: string;
  bytecode?: string;
  
  // Table changes
  handle?: string;
  key?: string;
  value?: any;
}

type ChangeType =
  | "write_resource"
  | "delete_resource"
  | "write_module"
  | "delete_module"
  | "write_table_item"
  | "delete_table_item";
```

---

## 6. Query Parameters

### 6.1 Standard Parameters

| Parameter | Type | Description | Default |
|-----------|------|-------------|---------|
| `ledger_version` | string (u64) | Query at specific version | Latest |
| `start` | string | Pagination cursor | First page |
| `limit` | number | Page size | 25 |
| `include` | string[] | Additional fields | None |
| `format` | `json` \| `bcs` | Response format | `bcs` |

### 6.2 Include Parameter

The `include` parameter allows clients to request additional data:

```
GET /v2/transactions?include=events,changes
GET /v2/accounts/{address}/resources?include=state_key_hash,size
```

**Available include values by endpoint:**

| Endpoint | Include Options |
|----------|-----------------|
| `/v2/transactions` | `events`, `changes`, `payload` |
| `/v2/accounts/{}/resources` | `state_key_hash`, `size` |
| `/v2/blocks/{height}` | `transactions` |

---

## 7. Batch Operations

### 7.1 Generic Batch Endpoint

```
POST /v2/batch
```

**Request Body:**
```typescript
interface BatchRequest {
  operations: Array<{
    id: string;           // Client correlation ID
    method: string;       // Operation name
    params: any;          // Operation parameters
  }>;
}
```

**Example:**
```json
{
  "operations": [
    {
      "id": "balance-1",
      "method": "get_account_balance",
      "params": {
        "address": "0x1",
        "asset_type": "0x1::aptos_coin::AptosCoin"
      }
    },
    {
      "id": "balance-2",
      "method": "get_account_balance",
      "params": {
        "address": "0x2",
        "asset_type": "0x1::aptos_coin::AptosCoin"
      }
    }
  ]
}
```

**Response:**
```typescript
interface BatchResponse {
  results: Array<{
    id: string;
    success: boolean;
    data?: any;
    error?: ApiError;
  }>;
}
```

### 7.2 Batch Limits

| Limit | Value |
|-------|-------|
| Max operations per batch | 100 |
| Max total request size | 10 MB |
| Timeout per operation | 5s |
| Total batch timeout | 30s |

---

## 8. Streaming API

### 8.1 Transaction Stream

```
GET /v2/transactions/stream
Accept: application/x-ndjson
```

**Parameters:**
- `start` (query): Starting version
- `limit` (query): Max transactions to stream

**Response:** Newline-delimited JSON stream
```
{"version":"1","hash":"0x...","type":"user_transaction",...}\n
{"version":"2","hash":"0x...","type":"block_metadata",...}\n
...
```

### 8.2 Event Subscription (SSE)

```
GET /v2/events/subscribe
Accept: text/event-stream
```

**Parameters:**
- `event_type` (query, optional): Filter by event type
- `sender` (query, optional): Filter by sender address

**Response:** Server-Sent Events
```
event: transaction
data: {"version":"123","hash":"0x...",...}

event: transaction
data: {"version":"124","hash":"0x...",...}
```

### 8.3 WebSocket API

```
WS /v2/ws
```

**Subscribe Message:**
```json
{
  "type": "subscribe",
  "channel": "transactions",
  "filters": {
    "sender": "0x1"
  }
}
```

**Unsubscribe Message:**
```json
{
  "type": "unsubscribe",
  "channel": "transactions"
}
```

---

## 9. Authentication & Rate Limiting

### 9.1 API Keys (Optional)

For higher rate limits, clients can use API keys:

```
GET /v2/transactions
X-Aptos-Api-Key: ak_xxxxxxxxxxxxx
```

### 9.2 Rate Limits

| Tier | Requests/Second | Burst |
|------|-----------------|-------|
| Anonymous | 10 | 50 |
| Basic API Key | 100 | 500 |
| Premium API Key | 1000 | 5000 |

### 9.3 Rate Limit Headers

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1234567890
```

---

## 10. Migration Guide

### 10.1 URL Changes

| v1 Endpoint | v2 Endpoint |
|-------------|-------------|
| `/v1/accounts/{address}` | `/v2/accounts/{address}` |
| `/v1/transactions` | `/v2/transactions` |
| `/v1/transactions/by_hash/{hash}` | `/v2/transactions/by_hash/{hash}` |
| `/v1/blocks/by_height/{height}` | `/v2/blocks/by_height/{height}` |
| `/v1/view` | `/v2/view` |
| `/v1/estimate_gas_price` | `/v2/gas/estimate` |
| `/v1/-/healthy` | `/v2/health/ready` |

### 10.2 Response Format Changes

**v1:**
```json
{
  "type": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
  "data": { "coin": { "value": "1000000" } }
}
```

**v2:**
```json
{
  "data": {
    "type": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
    "data": { "coin": { "value": "1000000" } }
  },
  "ledger_info": {
    "chain_id": 1,
    "ledger_version": "12345678",
    "ledger_timestamp_usec": "1234567890000000"
  }
}
```

### 10.3 SDK Updates Required

1. Update base URL from `/v1` to `/v2`
2. Handle new response envelope
3. Update Content-Type handling for BCS default
4. Implement new error handling
5. Add support for `include` parameter

### 10.4 Deprecation Timeline

| Phase | Timeline | Action |
|-------|----------|--------|
| v2 Beta | Month 1-2 | v2 available alongside v1 |
| v2 GA | Month 3 | v2 becomes recommended |
| v1 Deprecation | Month 6 | v1 marked deprecated |
| v1 Sunset | Month 12+ | v1 removed (with notice) |

---

## Appendix: OpenAPI Spec Generation

The v2 API will generate an OpenAPI 3.1 specification:

```yaml
openapi: 3.1.0
info:
  title: Aptos Node API v2
  version: 2.0.0
  description: |
    The Aptos Node API v2 provides a RESTful interface for interacting
    with the Aptos blockchain. It features BCS-first encoding, batch
    operations, and streaming support.
servers:
  - url: https://fullnode.mainnet.aptoslabs.com/v2
    description: Mainnet
  - url: https://fullnode.testnet.aptoslabs.com/v2
    description: Testnet
```
