# Aptos REST API Improvement Documents

This directory contains detailed technical documents for improving the Aptos REST API performance and evolving to a v2 architecture.

## Document Index

| Document | Description | Timeline | Risk |
|----------|-------------|----------|------|
| [01_SHORT_TERM_PERFORMANCE.md](./01_SHORT_TERM_PERFORMANCE.md) | Quick wins: caching, batching, metrics | 2-4 weeks | Low |
| [02_MEDIUM_TERM_PERFORMANCE.md](./02_MEDIUM_TERM_PERFORMANCE.md) | Streaming, parallelization, advanced caching | 4-12 weeks | Medium |
| [03_V2_API_DESIGN.md](./03_V2_API_DESIGN.md) | Complete v2 API specification | 3-6 months | Medium |
| [04_FRAMEWORK_MIGRATION.md](./04_FRAMEWORK_MIGRATION.md) | Poem to Axum framework migration | 4-8 weeks | Medium |

## Quick Links

### Short-Term Performance (Start Here)
- [Response Caching Layer](./01_SHORT_TERM_PERFORMANCE.md#1-response-caching-layer)
- [Batch Timestamp Lookups](./01_SHORT_TERM_PERFORMANCE.md#2-batch-timestamp-lookups)
- [Additional Metrics](./01_SHORT_TERM_PERFORMANCE.md#5-additional-metrics)

### Medium-Term Performance
- [Streaming Responses](./02_MEDIUM_TERM_PERFORMANCE.md#1-streaming-responses)
- [Parallel Transaction Rendering](./02_MEDIUM_TERM_PERFORMANCE.md#2-parallel-transaction-rendering)
- [View Function Caching](./02_MEDIUM_TERM_PERFORMANCE.md#3-view-function-result-caching)

### V2 API Design
- [Endpoint Specification](./03_V2_API_DESIGN.md#4-endpoint-specification)
- [Type Definitions](./03_V2_API_DESIGN.md#5-type-definitions)
- [Batch Operations](./03_V2_API_DESIGN.md#7-batch-operations)
- [Migration Guide](./03_V2_API_DESIGN.md#10-migration-guide)

### Framework Migration
- [Why Axum?](./04_FRAMEWORK_MIGRATION.md#3-migration-rationale)
- [Code Examples](./04_FRAMEWORK_MIGRATION.md#6-code-migration-examples)
- [OpenAPI with Utoipa](./04_FRAMEWORK_MIGRATION.md#7-openapi-generation)

## Summary

### Current State
- **Framework**: Poem with poem-openapi
- **Serialization**: JSON (default) and BCS
- **Pain Points**: Type annotation overhead, sequential processing, limited streaming

### Recommended Path

```
┌─────────────────────────────────────────────────────────────┐
│  Phase 1: Quick Wins (Weeks 1-4)                            │
│  - Add caching layer for hot resources                      │
│  - Batch timestamp lookups                                  │
│  - Add performance metrics                                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 2: Performance Foundations (Weeks 5-12)              │
│  - Parallel transaction rendering                           │
│  - View function caching                                    │
│  - Streaming response support                               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 3: V2 API + Framework (Months 3-6)                   │
│  - Migrate to Axum framework                                │
│  - Implement v2 endpoints                                   │
│  - Add batch operations                                     │
│  - WebSocket subscriptions                                  │
└─────────────────────────────────────────────────────────────┘
```

## Key Metrics to Track

| Metric | Current | Target |
|--------|---------|--------|
| P50 Latency (get_account) | ~5ms | <2ms |
| P99 Latency (get_transactions, 100 txns) | ~200ms | <50ms |
| Cache Hit Rate (resources) | 0% | >60% |
| Memory per Large Request | ~100MB | <10MB |

## Getting Started

1. **Understand the codebase**: Read `api/src/context.rs` and `api/src/transactions.rs`
2. **Start with metrics**: Implement the metrics from [01_SHORT_TERM_PERFORMANCE.md#5](./01_SHORT_TERM_PERFORMANCE.md#5-additional-metrics)
3. **Add caching**: Follow the caching implementation guide
4. **Benchmark**: Use `wrk2` to measure before/after

## Related Resources

- [Main Analysis Document](../API_PERFORMANCE_AND_V2_PLAN.md)
- [API Source Code](../../src/)
- [API Types](../../types/src/)
- [API Configuration](../../../config/src/config/api_config.rs)
