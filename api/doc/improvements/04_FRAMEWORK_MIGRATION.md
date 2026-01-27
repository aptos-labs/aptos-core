# API Framework Migration Analysis: Poem to Axum

## Overview

This document analyzes the current Poem-based API implementation and evaluates migration to alternative Rust web frameworks, with a focus on Axum as the primary candidate.

**Current Framework**: Poem 3.x  
**Recommended Target**: Axum 0.7+  
**Estimated Migration Effort**: 4-8 weeks

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Framework Comparison](#2-framework-comparison)
3. [Migration Rationale](#3-migration-rationale)
4. [Axum Architecture Overview](#4-axum-architecture-overview)
5. [Migration Strategy](#5-migration-strategy)
6. [Code Migration Examples](#6-code-migration-examples)
7. [OpenAPI Generation](#7-openapi-generation)
8. [Testing Strategy](#8-testing-strategy)
9. [Risk Assessment](#9-risk-assessment)
10. [Implementation Plan](#10-implementation-plan)

---

## 1. Current State Analysis

### 1.1 Poem Framework Usage

The current API uses Poem with the following features:

```rust
// Key dependencies in api/Cargo.toml
poem = "3"
poem-openapi = "5"
```

**Features Used:**
- `poem_openapi::OpenApi` - OpenAPI specification generation
- `poem_openapi::ApiResponse` - Response type generation
- `poem_openapi::param::{Path, Query}` - Parameter extraction
- `poem_openapi::payload::Json` - JSON payloads
- `poem::middleware::*` - Middleware stack (CORS, compression, etc.)
- `poem::listener::TcpListener` - HTTP server

### 1.2 Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Poem Server                            │
├─────────────────────────────────────────────────────────────┤
│  Middleware: CORS → Compression → SizeLimit → CatchPanic    │
├─────────────────────────────────────────────────────────────┤
│  Route: /v1 → OpenApiService(APIs tuple)                    │
├─────────────────────────────────────────────────────────────┤
│  APIs: AccountsApi, TransactionsApi, BlocksApi, ...         │
├─────────────────────────────────────────────────────────────┤
│  Handlers: async fn with spawn_blocking for DB ops          │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 Pain Points with Current Implementation

| Issue | Description | Impact |
|-------|-------------|--------|
| **OpenAPI Coupling** | Poem-OpenAPI tightly couples routing and spec generation | Hard to customize spec |
| **Macro Heavy** | Heavy reliance on proc macros for response types | Slower compilation |
| **Limited Ecosystem** | Smaller community than Axum/Actix | Fewer middleware options |
| **Error Handling** | Custom error response macros are complex | Maintenance burden |
| **Testing** | poem-test exists but less mature | Testing friction |

---

## 2. Framework Comparison

### 2.1 Candidate Frameworks

| Framework | Stars | Downloads/month | Last Release | Maintained |
|-----------|-------|-----------------|--------------|------------|
| **Axum** | 18k+ | 2M+ | Active | Yes (Tokio team) |
| **Actix-web** | 21k+ | 1.5M+ | Active | Yes |
| **Poem** | 3k+ | 200k+ | Active | Yes |
| **Warp** | 9k+ | 500k+ | Slower | Partially |

### 2.2 Feature Comparison

| Feature | Poem | Axum | Actix-web |
|---------|------|------|-----------|
| OpenAPI Generation | Built-in (poem-openapi) | Via utoipa | Via paperclip |
| Performance | Good | Excellent | Excellent |
| Type Safety | Good | Excellent | Good |
| Middleware | Good | Excellent (tower) | Custom |
| Async Runtime | Tokio | Tokio | Actix-rt or Tokio |
| Learning Curve | Moderate | Moderate | Steeper |
| Community Size | Small | Large | Large |
| Tower Compatibility | Partial | Native | Limited |

### 2.3 Performance Benchmarks

Based on TechEmpower benchmarks and community testing:

| Framework | JSON Serialization | DB Queries | Composite |
|-----------|-------------------|------------|-----------|
| Axum | ~95% of max | ~90% of max | Excellent |
| Actix-web | ~98% of max | ~92% of max | Excellent |
| Poem | ~90% of max | ~88% of max | Good |

*Note: Performance differences are often negligible for real-world APIs.*

---

## 3. Migration Rationale

### 3.1 Why Migrate to Axum?

**1. Tower Ecosystem Integration**
- Native Tower middleware support
- Reusable middleware across services
- Battle-tested components (timeout, rate limiting, tracing)

**2. Better Type-Driven Development**
- Extractors that leverage Rust's type system
- Compile-time route checking
- Clear ownership semantics

**3. Stronger Community**
- More third-party middleware and integrations
- Better documentation and examples
- Faster bug fixes and feature development

**4. Maintained by Tokio Team**
- Long-term stability guaranteed
- Aligned with Tokio ecosystem evolution
- First-class async support

**5. Superior Error Handling**
- Result-based error handling
- IntoResponse trait for flexible responses
- Better error composition

### 3.2 Why Not Migrate?

| Concern | Mitigation |
|---------|------------|
| Migration effort | Gradual migration strategy |
| OpenAPI generation | utoipa provides good support |
| Breaking changes | v2 API allows clean break |
| Team familiarity | Axum is well-documented |

---

## 4. Axum Architecture Overview

### 4.1 Core Concepts

```rust
use axum::{
    Router,
    routing::{get, post},
    extract::{Path, Query, State, Json},
    response::IntoResponse,
    middleware,
};

// Router with state
let app = Router::new()
    .route("/accounts/:address", get(get_account))
    .route("/transactions", post(submit_transaction))
    .layer(middleware::from_fn(logging))
    .with_state(app_state);

// Handler signature
async fn get_account(
    State(ctx): State<Arc<Context>>,
    Path(address): Path<Address>,
    Query(params): Query<AccountParams>,
) -> Result<Json<AccountData>, ApiError> {
    // ...
}
```

### 4.2 Extractor Pattern

Axum uses extractors to pull data from requests:

```rust
// Custom extractor for Accept header
pub struct AcceptType(pub ResponseFormat);

#[async_trait]
impl<S> FromRequestParts<S> for AcceptType
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let accept = parts
            .headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json");
        
        let format = match accept {
            "application/x-bcs" => ResponseFormat::Bcs,
            _ => ResponseFormat::Json,
        };
        
        Ok(AcceptType(format))
    }
}
```

### 4.3 Response Types

```rust
use axum::response::{IntoResponse, Response};

// Custom response type
pub struct ApiResponse<T> {
    data: T,
    ledger_info: LedgerInfo,
    status: StatusCode,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let body = serde_json::to_string(&ApiEnvelope {
            data: self.data,
            ledger_info: self.ledger_info.into(),
        }).unwrap();
        
        Response::builder()
            .status(self.status)
            .header("Content-Type", "application/json")
            .header("X-Aptos-Ledger-Version", self.ledger_info.version.to_string())
            .body(body.into())
            .unwrap()
    }
}
```

### 4.4 Error Handling

```rust
use axum::response::{IntoResponse, Response};

pub enum ApiError {
    NotFound { resource: String, identifier: String },
    BadRequest { message: String },
    Internal { message: String },
    VmError { status: StatusCode, message: String },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            ApiError::NotFound { resource, identifier } => (
                StatusCode::NOT_FOUND,
                json!({
                    "error_code": "NOT_FOUND",
                    "message": format!("{} not found: {}", resource, identifier),
                }),
            ),
            ApiError::BadRequest { message } => (
                StatusCode::BAD_REQUEST,
                json!({
                    "error_code": "INVALID_INPUT",
                    "message": message,
                }),
            ),
            // ... other variants
        };
        
        (status, Json(body)).into_response()
    }
}
```

---

## 5. Migration Strategy

### 5.1 Phased Approach

```
Phase 1: Foundation (Week 1-2)
├── Set up Axum alongside Poem
├── Create shared types and extractors
├── Implement core middleware
└── Set up testing infrastructure

Phase 2: Core Endpoints (Week 3-4)
├── Migrate health/info endpoints
├── Migrate accounts endpoints
├── Migrate state endpoints
└── Verify OpenAPI generation

Phase 3: Transaction Endpoints (Week 5-6)
├── Migrate transaction queries
├── Migrate transaction submission
├── Migrate simulation
└── Performance testing

Phase 4: Advanced Features (Week 7-8)
├── Migrate view functions
├── Implement streaming (new)
├── Implement batch endpoints (new)
└── Full integration testing

Phase 5: Cutover (Week 9)
├── Switch default to Axum
├── Deprecate Poem routes
├── Monitor and fix issues
└── Documentation update
```

### 5.2 Parallel Running Strategy

Run both frameworks simultaneously during migration:

```rust
// api/src/runtime.rs

pub fn bootstrap(config: &NodeConfig, ...) -> anyhow::Result<Runtime> {
    let runtime = aptos_runtimes::spawn_named_runtime("api", workers);
    let context = Arc::new(Context::new(...));
    
    // Poem (v1 API)
    if config.api.v1_enabled {
        attach_poem_to_runtime(runtime.handle(), context.clone(), config)?;
    }
    
    // Axum (v2 API)
    if config.api.v2_enabled {
        attach_axum_to_runtime(runtime.handle(), context.clone(), config)?;
    }
    
    Ok(runtime)
}

fn attach_axum_to_runtime(
    handle: &Handle,
    context: Arc<Context>,
    config: &NodeConfig,
) -> anyhow::Result<SocketAddr> {
    let app = create_axum_router(context);
    
    let listener = TcpListener::bind(&config.api.v2_address).await?;
    
    handle.spawn(async move {
        axum::serve(listener, app).await
    });
    
    Ok(config.api.v2_address)
}
```

---

## 6. Code Migration Examples

### 6.1 Endpoint Migration

**Poem (Current):**

```rust
#[derive(Clone)]
pub struct AccountsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl AccountsApi {
    #[oai(
        path = "/accounts/:address",
        method = "get",
        operation_id = "get_account",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account(
        &self,
        accept_type: AcceptType,
        address: Path<Address>,
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<AccountData> {
        fail_point_poem("endpoint_get_account")?;
        self.context.check_api_output_enabled("Get account", &accept_type)?;

        let context = self.context.clone();
        api_spawn_blocking(move || {
            let account = Account::new(context, address.0, ledger_version.0, None, None)?;
            account.account(&accept_type)
        })
        .await
    }
}
```

**Axum (Target):**

```rust
// api/src/v2/accounts.rs

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router,
};

pub fn accounts_router() -> Router<Arc<Context>> {
    Router::new()
        .route("/:address", get(get_account))
        .route("/:address/resources", get(get_account_resources))
        .route("/:address/modules", get(get_account_modules))
        .route("/:address/balance/:asset_type", get(get_account_balance))
}

#[utoipa::path(
    get,
    path = "/v2/accounts/{address}",
    params(
        ("address" = String, Path, description = "Account address"),
        ("ledger_version" = Option<u64>, Query, description = "Ledger version")
    ),
    responses(
        (status = 200, description = "Account data", body = ApiResponse<AccountData>),
        (status = 404, description = "Account not found", body = ApiError),
    ),
    tag = "accounts"
)]
async fn get_account(
    State(ctx): State<Arc<Context>>,
    Path(address): Path<Address>,
    Query(params): Query<AccountParams>,
    accept: AcceptType,
) -> Result<ApiResponse<AccountData>, ApiError> {
    let ledger_version = params.ledger_version.map(|v| v.0);
    
    let result = tokio::task::spawn_blocking(move || {
        let (ledger_info, version) = ctx.get_latest_ledger_info_and_verify_lookup_version(ledger_version)?;
        
        let account_resource = ctx.get_resource::<AccountResource>(
            address.into(),
            version,
        )?;
        
        match account_resource {
            Some(resource) => Ok((AccountData::from(resource), ledger_info)),
            None => Err(ApiError::not_found("Account", address.to_string())),
        }
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))??;
    
    let (data, ledger_info) = result;
    
    Ok(ApiResponse::new(data, ledger_info, accept))
}

#[derive(Deserialize)]
pub struct AccountParams {
    ledger_version: Option<U64>,
}
```

### 6.2 Middleware Migration

**Poem (Current):**

```rust
let route = Route::new()
    .nest("/v1", api_service)
    .with(cors)
    .with_if(config.api.compression_enabled, Compression::new())
    .with(PostSizeLimit::new(size_limit))
    .with(CatchPanic::new().with_handler(panic_handler))
    .catch_all_error(convert_error)
    .around(middleware_log);
```

**Axum (Target):**

```rust
use axum::{
    middleware,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
    catch_panic::CatchPanicLayer,
};

fn create_axum_router(context: Arc<Context>, config: &ApiConfig) -> Router {
    let middleware_stack = ServiceBuilder::new()
        // Tracing (logging)
        .layer(TraceLayer::new_for_http())
        // Panic handling
        .layer(CatchPanicLayer::custom(handle_panic))
        // Request body size limit
        .layer(RequestBodyLimitLayer::new(config.content_length_limit as usize))
        // Compression (conditional)
        .option_layer(config.compression_enabled.then(CompressionLayer::new))
        // CORS
        .layer(
            CorsLayer::new()
                .allow_credentials(true)
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(Any)
        )
        // Custom metrics middleware
        .layer(middleware::from_fn(metrics_middleware));

    Router::new()
        .nest("/v2", v2_routes())
        .layer(middleware_stack)
        .with_state(context)
}

async fn metrics_middleware(
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    let status = response.status().as_u16().to_string();
    
    metrics::HISTOGRAM
        .with_label_values(&[method.as_str(), &path, &status])
        .observe(duration.as_secs_f64());
    
    response
}
```

### 6.3 Custom Extractor Migration

**Poem AcceptType:**

```rust
// Current Poem implementation
pub enum AcceptType {
    Json,
    Bcs,
}

impl<'a> FromRequest<'a> for AcceptType {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> poem::Result<Self> {
        // ...
    }
}
```

**Axum AcceptType:**

```rust
// api/src/v2/extractors.rs

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};

#[derive(Clone, Copy, Debug)]
pub enum AcceptType {
    Json,
    Bcs,
}

impl Default for AcceptType {
    fn default() -> Self {
        AcceptType::Bcs  // v2 defaults to BCS
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AcceptType
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let accept = parts
            .headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/x-bcs");
        
        let format = if accept.contains("application/json") {
            AcceptType::Json
        } else {
            AcceptType::Bcs
        };
        
        Ok(format)
    }
}
```

---

## 7. OpenAPI Generation

### 7.1 Utoipa Setup

```toml
# api/Cargo.toml
[dependencies]
utoipa = { version = "4", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }
```

### 7.2 API Documentation

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Aptos Node API v2",
        version = "2.0.0",
        description = "RESTful API for interacting with the Aptos blockchain",
        license(name = "Apache 2.0", url = "https://www.apache.org/licenses/LICENSE-2.0.html"),
        contact(name = "Aptos Labs", url = "https://github.com/aptos-labs/aptos-core")
    ),
    servers(
        (url = "/v2", description = "API v2 endpoints")
    ),
    paths(
        accounts::get_account,
        accounts::get_account_resources,
        accounts::get_account_modules,
        transactions::get_transactions,
        transactions::submit_transaction,
        // ... more paths
    ),
    components(
        schemas(
            AccountData,
            TransactionV2,
            ApiError,
            // ... more schemas
        )
    ),
    tags(
        (name = "accounts", description = "Account operations"),
        (name = "transactions", description = "Transaction operations"),
        (name = "blocks", description = "Block operations"),
        (name = "events", description = "Event operations"),
        (name = "state", description = "State queries"),
        (name = "view", description = "View function execution"),
    )
)]
pub struct ApiDoc;

// Add Swagger UI route
fn create_axum_router(context: Arc<Context>) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/v2/swagger-ui").url("/v2/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/v2", v2_routes())
        .with_state(context)
}
```

### 7.3 Schema Derivation

```rust
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccountData {
    /// Account address
    #[schema(example = "0x1")]
    pub address: String,
    
    /// Current sequence number
    #[schema(example = "42")]
    pub sequence_number: String,
    
    /// Authentication key
    #[schema(example = "0x...")]
    pub authentication_key: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Machine-readable error code
    #[schema(example = "NOT_FOUND")]
    pub error_code: String,
    
    /// Human-readable error message
    #[schema(example = "Account not found")]
    pub message: String,
    
    /// Request ID for debugging
    #[schema(example = "req_abc123")]
    pub request_id: String,
}
```

---

## 8. Testing Strategy

### 8.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_get_account() {
        let ctx = create_test_context();
        let app = create_axum_router(ctx);
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v2/accounts/0x1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let account: ApiResponse<AccountData> = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(account.data.address, "0x1");
    }
    
    #[tokio::test]
    async fn test_account_not_found() {
        let ctx = create_test_context();
        let app = create_axum_router(ctx);
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v2/accounts/0xdeadbeef")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
```

### 8.2 Integration Tests

```rust
// api/tests/axum_integration.rs

use aptos_api_test_context::TestContext;

#[tokio::test]
async fn test_transaction_submission() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();
    
    // Submit transaction
    let response = client
        .post(&format!("{}/v2/transactions/submit", ctx.api_url))
        .header("Content-Type", "application/x-bcs")
        .body(create_test_transaction())
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 202);
    
    let result: SubmitResult = response.json().await.unwrap();
    assert!(!result.hash.is_empty());
}
```

---

## 9. Risk Assessment

### 9.1 Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing clients | Medium | High | Parallel running, v2 prefix |
| Performance regression | Low | Medium | Extensive benchmarking |
| OpenAPI incompatibility | Low | Medium | Test with SDK generators |
| Migration delays | Medium | Medium | Phased approach |
| Hidden Poem dependencies | Low | Low | Thorough code review |

### 9.2 Rollback Plan

1. Keep Poem implementation intact during migration
2. Use feature flags to enable/disable Axum routes
3. Monitor error rates and latency during rollout
4. Quick switch back to Poem if issues arise

---

## 10. Implementation Plan

### 10.1 Week-by-Week Schedule

| Week | Focus | Deliverables |
|------|-------|--------------|
| 1 | Setup | Axum scaffolding, extractors, error types |
| 2 | Core | Health, accounts, basic state endpoints |
| 3 | Transactions | Query endpoints, response types |
| 4 | Transactions | Submit, simulate, batch |
| 5 | Advanced | View functions, events |
| 6 | Features | Streaming, WebSocket |
| 7 | Testing | Integration tests, load tests |
| 8 | Polish | Documentation, OpenAPI, monitoring |

### 10.2 Success Criteria

- [ ] All v1 endpoints have v2 equivalents
- [ ] OpenAPI spec generates correctly
- [ ] Performance >= v1 (benchmark comparison)
- [ ] All tests passing
- [ ] SDK compatibility verified
- [ ] Documentation complete

### 10.3 Resource Requirements

| Role | Effort |
|------|--------|
| Backend Engineer | 6-8 weeks |
| API Review | 1 week |
| SDK Updates | 2 weeks (parallel) |
| Documentation | 1 week (parallel) |

---

## Appendix: Quick Reference

### Poem to Axum Cheat Sheet

| Poem | Axum |
|------|------|
| `#[OpenApi]` | `#[utoipa::path]` |
| `Path<T>` | `Path<T>` |
| `Query<T>` | `Query<T>` |
| `Json<T>` | `Json<T>` |
| `Data<T>` | `State<T>` |
| `poem::Result` | `Result<impl IntoResponse, E>` |
| `poem::get(handler)` | `routing::get(handler)` |
| `Route::new().nest()` | `Router::new().nest()` |
| `.with(middleware)` | `.layer(middleware)` |

### Key Dependencies

```toml
[dependencies]
axum = "0.7"
axum-extra = { version = "0.9", features = ["typed-header"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "compression", "trace", "limit", "catch-panic"] }
utoipa = { version = "4", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }
```
