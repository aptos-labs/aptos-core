# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

This directory contains the `aptos-db-indexer` crate — the AptosDB **internal indexer**. Repo-wide commands (lint, formatting, framework rebuilds) are covered by the root `CLAUDE.md`; this file covers what is specific to this crate.

## Commands

```bash
cargo check -p aptos-db-indexer     # Quick compile check
cargo build -p aptos-db-indexer
cargo test -p aptos-db-indexer      # Note: crate has no in-crate unit tests (see Testing below)
```

New Rust files here use the Innovation-Enabling license header (`Copyright (c) Aptos Foundation` + license URL), matching existing files.

## Purpose

This crate builds **secondary indices** over the main AptosDB ledger so that queries the ledger schema can't serve (events by key, transactions by account, state keys by prefix, table-handle type info) stay off the storage critical path. It maintains two separate RocksDB databases, opened via `db_ops.rs`:

1. **Internal indexer DB** (`internal_indexer_db`) — owned by `DBIndexer` in `db_indexer.rs`
2. **Table info DB** — owned by `IndexerAsyncV2` in `db_v2.rs`

All schemas (column families, key/value codecs) live in the sibling crate `storage/indexer_schemas` (`aptos-db-indexer-schemas`). **Adding or changing an index means editing that crate first** (schema definition plus the `column_families()` / `internal_indexer_column_families()` lists), then the write/read paths here.

Consumption note: everything `DBIndexer` builds is served exclusively through the **JSON REST API** (`api/`). The gRPC transaction stream (`ecosystem/indexer-grpc/indexer-grpc-fullnode`, enabled via `indexer_grpc.enabled`, off by default) is a separate serving surface that consumes only the table-info half of this crate (to resolve table handles when converting transactions to protobuf); it does not use the `DBIndexer` indices.

## Architecture

### Write path (`db_indexer.rs`)

`DBIndexer::process(start, end)` tail-follows the main DB: it zips the transaction, event, and write-set iterators from `DbReader`, builds one `SchemaBatch` per batch (size from config), and hands it to a dedicated committer thread (`DBCommitter`) over an mpsc channel.

- **Writes are asynchronous.** `process_a_batch` returns before data is durable. Anything that reads back its own writes (tests especially) must call `DBIndexer::flush()` first.
- Each enabled index records its own progress version under `InternalIndexerMetadataSchema` (`MetadataKey::{LatestVersion, EventVersion, StateVersion, TransactionVersion, EventV2TranslationVersion}`) in the same batch, so progress is atomic with the indexed data.
- Which indices are populated is gated per-flag by `InternalIndexerDBConfig` (`config/src/config/internal_indexer_db_config.rs`): `enable_transaction`, `enable_event`, `enable_event_v2_translation`, `enable_statekeys`, `batch_size`.

The driver loop lives outside this crate: `InternalIndexerDBService` in `ecosystem/indexer-grpc/indexer-grpc-table-info/src/internal_indexer_db_service.rs` (the crate name reflects the ecosystem tree it sits in, not gRPC serving), wired up in `aptos-node/src/services.rs`. The backup/restore path (`storage/backup/backup-cli`) writes state keys directly via `InternalIndexerDB::write_keys_to_indexer_db`.

### Event V2→V1 translation (`event_v2_translator.rs`)

Module events (`ContractEventV2`) carry no event key or sequence number, but old API queries are keyed on them. `EventV2TranslationEngine` holds a registry of per-`TypeTag` `EventV2Translator` impls (coin deposit/withdraw, token/collection events, etc.). Each translator reconstructs the V1 `EventKey` and sequence number by reading the corresponding on-chain resource's event handle from the main DB's latest state checkpoint view, then the result is stored in `TranslatedV1EventSchema` and mirrored into the event-by-key/version indices.

- Sequence numbers are tracked in an in-memory `DashMap` cache backed by `EventSequenceNumberSchema`; `get_next_sequence_number` falls back to the on-chain handle count.
- Translation failure is non-fatal: the event is skipped (`Ok(None)`) with a warning. "Resource not found" for Mint/Burn is expected and silently ignored (ConcurrentSupply collections have no V1-style supply resource).

### Table info indexing (`db_v2.rs`)

`IndexerAsyncV2` maps `TableHandle` → `TableInfo` (key/value type tags) by running `AptosValueAnnotator` over write sets. Nested tables create an ordering problem — a table item may appear before its parent's type info is known — handled by parking raw bytes in the `pending_on` map and re-parsing when the parent's info arrives. `get_table_info_with_retry` spins (10ms sleeps) because readers may race the async parser.

The driver (`TableInfoService`) also lives in `ecosystem/indexer-grpc/indexer-grpc-table-info`; it fetches transactions via the API `Context` and the fullnode stream coordinator machinery, gated by `IndexerTableInfoConfig` (`table_info_service_mode`, `parser_task_count`, `parser_batch_size`).

### Read path (`indexer_reader.rs`)

`IndexerReaders` implements the `IndexerReader` trait from `aptos_types::indexer::indexer_db_reader` over optional `IndexerAsyncV2` and `DBIndexer` handles; this is what the API layer consumes. Reads guard against serving data the indexer hasn't caught up to via `ensure_cover_ledger_version`. State-prefix queries (`utils.rs::PrefixedStateValueIterator`) iterate state keys from the indexer DB but fetch values from the main DB at the requested version.

## Testing

There are no unit tests in this crate. Coverage lives in:

- `execution/executor/tests/internal_indexer_test.rs` — end-to-end indexing over executed blocks
- `storage/db-tool/src/tests.rs` — restore-path tests
- `api/test-context` (`MockInternalIndexerDBService`) and `testsuite/smoke-test/src/fullnode.rs` — API-level and node-level integration

When writing tests that assert on indexed data, remember the async commit: call `flush()` on the `DBIndexer` before reading.
