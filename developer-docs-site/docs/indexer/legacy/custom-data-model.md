---
title: "Custom Data Model"
---

:::warning Legacy Indexer
This is documentation for the legacy indexer. To learn how to write a custom processor with the latest indexer stack, see [Custom Processors](/indexer/custom-processors).
:::

## Define your own data model

Use this method if you want to develop your custom indexer for the Aptos ledger data.

:::tip When to use custom indexer
Currently Aptos-provided indexing service (see above) supports the following core Move modules:
- `0x1::coin`.
- `0x3::token`.
- `0x3::token_transfers`.

If you need an indexed database for any other Move modules and contracts, then you should develop your custom indexer.
:::

Creating a custom indexer involves the following steps. Refer to the indexing block diagram at the start of this document.

1. Define new table schemas, using an ORM like [Diesel](https://diesel.rs/). In this document Diesel is used to describe the custom indexing steps ("Business logic" and the data queries in the diagram).
2. Create new data models based on the new tables ("Business logic" in the diagram).
3. Create a new transaction processor, or optionally add to an existing processor. In the diagram this step corresponds to processing the ledger database according to the new business logic and writing to the indexed database.
4. Integrate the new processor. Optional if you are reusing an existing processor.

In the below detailed description, an example of indexing and querying for the coin balances is used. You can see this in the [`coin_processor`](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/processors/coin_processor.rs).

### 1. Define new table schemas

In this example we use [PostgreSQL](https://www.postgresql.org/) and [Diesel](https://diesel.rs/) as the ORM. To make sure that we make backward-compatible changes without having to reset the database at every upgrade, we use [Diesel migrations](https://docs.rs/diesel_migrations/latest/diesel_migrations/) to manage the schema. This is why it is very important to start with generating a new Diesel migration before doing anything else.

Make sure you clone the Aptos-core repo by running `git clone https://github.com/aptos-labs/aptos-core.git` and then `cd` into `aptos-core/tree/main/crates/indexer` directory. Then proceed as below.

a. The first step is to create a new Diesel migration. This will generate a new folder under [migrations](https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer/migrations) with `up.sql` and `down.sql`

```bash
DATABASE_URL=postgres://postgres@localhost:5432/postgres diesel migration generate add_coin_tables
```

b. Create the necessary table schemas. This is just PostgreSQL code. In the code shown below, the `up.sql` will have the new changes and `down.sql` will revert those changes.

```sql
-- up.sql
-- coin balances for each version
CREATE TABLE coin_balances (
  transaction_version BIGINT NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  -- Hash of the non-truncated coin type
  coin_type_hash VARCHAR(64) NOT NULL,
  -- creator_address::name::symbol<struct>
  coin_type VARCHAR(5000) NOT NULL,
  amount NUMERIC NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    owner_address,
    coin_type_hash
  )
);
-- latest coin balances
CREATE TABLE current_coin_balances {...}
-- down.sql
DROP TABLE IF EXISTS coin_balances;
DROP TABLE IF EXISTS current_coin_balances;
```

See the [full source for `up.sql` and `down.sql`](https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer/migrations/2022-10-04-073529_add_coin_tables).

c. Run the migration. We suggest running it multiple times with `redo` to ensure that both `up.sql` and `down.sql` are implemented correctly. This will also modify the [`schema.rs`](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/schema.rs) file.

```bash
DATABASE_URL=postgres://postgres@localhost:5432/postgres diesel migration run
DATABASE_URL=postgres://postgres@localhost:5432/postgres diesel migration redo
```

### 2. Create new data schemas

We now have to prepare the Rust data models that correspond to the Diesel schemas. In the case of coin balances, we will define `CoinBalance` and `CurrentCoinBalance` as below:

```rust
#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, owner_address, coin_type))]
#[diesel(table_name = coin_balances)]
pub struct CoinBalance {
    pub transaction_version: i64,
    pub owner_address: String,
    pub coin_type_hash: String,
    pub coin_type: String,
    pub amount: BigDecimal,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(owner_address, coin_type))]
#[diesel(table_name = current_coin_balances)]
pub struct CurrentCoinBalance {
    pub owner_address: String,
    pub coin_type_hash: String,
    pub coin_type: String,
    pub amount: BigDecimal,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
}
```

We will also need to specify the parsing logic, where the input is a portion of the transaction. In the case of coin balances, we can find all the details in `WriteSetChanges`, specifically where the write set change type is `write_resources`.

**Where to find the relevant data for parsing**: This requires a combination of understanding the Move module and the structure of the transaction. In the example of coin balance, the contract lives in [coin.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move), specifically the coin struct (search for `struct Coin`) that has a `value` field. We then look at an [example transaction](https://fullnode.testnet.aptoslabs.com/v1/transactions/by_version/259518) where we find this exact structure in `write_resources`:

```json
"changes": [
  {
    ...
    "data": {
      "type": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
      "data": {
        "coin": {
          "value": "49742"
      },
      ...
```

See the full code in [coin_balances.rs](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/models/coin_models/coin_balances.rs).

### 3. Create a new processor

Now that we have the data model and the parsing function, we need to call that parsing function and save the resulting model in our Postgres database. We do this by creating (or modifying) a `processor`. We have abstracted a lot already from that class, so the only function that should be implemented is `process_transactions` (there are a few more functions that should be copied, those should be obvious from the example).

The `process_transactions` function takes in a vector of transactions with a start and end version that are used for tracking purposes. The general flow should be:
  - Loop through transactions in the vector.
  - Aggregate relevant models. Sometimes deduping is required, e.g. in the case of `CurrentCoinBalance`.
  - Insert the models into the database in a single Diesel transaction. This is important, to ensure that we do not have partial writes.
  - Return status (error or success).

:::tip Coin transaction processor
See [coin_process.rs](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/processors/coin_processor.rs) for a relatively straightforward example. You can search for `coin_balances` in the page for the specific code snippet related to coin balances.
:::

**How to decide whether to create a new processor:** This is completely up to you. The benefit of creating a new processor is that you are starting from scratch so you will have full control over exactly what gets written to the indexed database. The downside is that you will have to maintain a new fullnode, since there is a 1-to-1 mapping between a fullnode and the processor.

### 4. Integrate the new processor

This is the easiest step and involves just a few additions.

1. To start with, make sure to add the new processor in the Rust code files: [`mod.rs`](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/processors/mod.rs) and [`runtime.rs`](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/runtime.rs). See below:

[**mod.rs**](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/processors/mod.rs)

```rust
pub enum Processor {
  CoinProcessor,
  ...
}
...
  COIN_PROCESSOR_NAME => Self::CoinProcessor,
```

[**runtime.rs**](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/runtime.rs)

```rust
Processor::CoinProcessor => Arc::new(CoinTransactionProcessor::new(conn_pool.clone())),
```

2. Create a `fullnode.yaml` with the correct configuration and test the custom indexer by starting a fullnode with this `fullnode.yaml`.

**fullnode.yaml**

```yaml
storage:
  enable_indexer: true
  storage_pruner_config:
      ledger_pruner_config:
          enable: false

indexer:
  enabled: true
  check_chain_id: true
  emit_every: 1000
  postgres_uri: "postgres://postgres@localhost:5432/postgres"
  processor: "coin_processor"
  fetch_tasks: 10
  processor_tasks: 10
```

Test by starting an Aptos fullnode by running the below command. You will see many logs in the terminal output, so use the `grep` filter to see only indexer log output, as shown below:

```bash
cargo run -p aptos-node --features "indexer" --release -- -f ./fullnode_coin.yaml | grep -E "_processor"
```

See the full instructions on how to start an indexer-enabled fullnode in [Indexer Fullnode](./indexer-fullnode).
