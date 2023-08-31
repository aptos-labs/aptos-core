---
title: "Migrate to Transaction Stream Service"
---

This guide contains information on how to migrate to using the Transaction Stream Service if you are currently running a legacy indexer.

The old indexer stack requires running an archival fullnode with additional threads to process the transactions which is difficult and expensive to maintain. Adding more custom logic either requires a bulkier machine, or running several fullnodes that scale linearly.

This new way of indexing uses the [Transaction Stream Service](https://aptos.dev/indexer/txn-stream/). You can either use the [Labs-Hosted Transaction Stream Service](https://aptos.dev/indexer/txn-stream/labs-hosted/) or [run your own instance of Transaction Stream Service](https://aptos.dev/indexer/txn-stream/self-hosted).

## 1. Clone the repo

```
# SSH
git clone git@github.com:aptos-labs/aptos-indexer-processors.git

# HTTPS
git clone https://github.com/aptos-labs/aptos-indexer-processors.git
```

Navigate to the directory for the service:

```
cd aptos-indexer-processors
cd rust/processor
```

## 2. Migrate processors to Transaction Stream Service

For each processor you're migrating, you'll need to create a config file using the template below. You can find more information about each field of the config file [here](https://aptos.dev/indexer/api/self-hosted/#configuration).

```yaml
health_check_port: 8084
server_config:
  processor_name: default_processor
  postgres_connection_string: <postgres_uri, e.g. postgresql://postgres:@localhost:5432/indexer>
  indexer_grpc_data_service_address: <url_from_api_gateway>
  indexer_grpc_http2_ping_interval_in_secs: 60
  indexer_grpc_http2_ping_timeout_in_secs: 10
  auth_token: <auto_token_from_api_gateway>
```

To connect the processor to the Transaction Stream Service, you need to set the URL for `indexer_grpc_data_service_address`. Choose one of the following options.

### Option A: Connect to Labs-Hosted Transaction Stream Service

The main benefit of using the Labs-Hosted Transaction Stream Service is that you no longer need to run an archival fullnode to get a stream of transactions. This service is rate-limited. Instructions to connect to Labs-Hosted Transaction Stream can be found [here](https://aptos.dev/indexer/txn-stream/labs-hosted).

### Option B: Run a Self-Hosted Transaction Stream Service

If you choose to, you can run a self-hosted instance of the Transaction Stream Service and connect your processors to it. Instructions to run a Self-Hosted Transaction Stream can be found [here](https://aptos.dev/indexer/txn-stream/self-hosted).

## 3. (Optional) Migrate custom processors to Transaction Stream Service

If you have custom processors written with the old indexer, we highly recommend starting from scratch with a new database. Using a new database ensures that all your custom database migrations will be applied during this migration.

### a. Migrate custom table schemas

Migrate your custom schemas by copying over each of your custom migrations to the [`migrations`](https://github.com/aptos-labs/aptos-indexer-processors/tree/main/rust/processor/migrations) folder.

### b. Migrate custom processors code

Migrate the code by copying over your custom processors to the [`processors`](https://github.com/aptos-labs/aptos-indexer-processors/tree/main/rust/processor) folder and any relevant custom models to the [`models`](https://github.com/aptos-labs/aptos-indexer-processors/tree/main/rust/processor/src/models) folder. Integrate the custom processors with the rest of the code by adding them to the following Rust code files.

[`mod.rs`](https://github.com/aptos-labs/aptos-indexer-processors/blob/main/rust/processor/src/processors/mod.rs)

```
pub enum Processor {
    ...
    CoinProcessor,
    ...
}

impl Processor {
    ...
    COIN_PROCESSOR_NAME => Self::CoinProcessor,
    ...
}
```

[`worker.rs`](https://github.com/aptos-labs/aptos-indexer-processors/blob/main/rust/processor/src/worker.rs)

```
Processor::CoinProcessor => {
    Arc::new(CoinTransactionProcessor::new(self.db_pool.clone()))
},
```

## 4. Backfill Postgres database with Diesel

Even though the new processors have the same Postgres schemas as the old ones, we recommend you do a complete backfill (ideally writing to a new DB altogether) because some fields are a bit different as a result of the protobuf conversion.

These instructions asusme you are familar with using [Diesel migrations](https://docs.rs/diesel_migrations/latest/diesel_migrations/). Run the full database migration with the following command:

```
DATABASE_URL=postgres://postgres@localhost:5432/postgres diesel migration run
```

## 5. Run the migrated processors

To run a single processor, use the following command:

```
cargo run --release -- -c config.yaml
```

If you have multiple processors, you'll need to run a separate instance of the service for each of the processors.

If you'd like to run the processor as a Docker image, the instructions are listed [here](https://aptos.dev/indexer/api/self-hosted#run-with-docker).

## FAQs

### 1. Will the protobuf ever be updated, and what do I need to do at that time?

The protobuf schema may be updated in the future. Backwards incompatible changes will be communicated in release notes.

### 2. What if I already have custom logic written in the old indexer? Is it easy to migrate those?

Since the new indexer stack has the same Postgres schema as the old indexer stack, it should be easy to migrate your processors. We still highly recommend creating a new DB for this migration so that any custom DB migrations are applie.

Follow Step 3 in this guide to migrate your custom logic over to the new processors stack.
