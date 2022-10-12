---
title: "Indexing"
slug: "indexing"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Indexing

## Concept

An application built on the Aptos blockchain, on any blockchain for that matter, requires that the raw data from the blockchain be shaped by the application-specific data model before the application can consume it. The [Aptos Node API](https://fullnode.devnet.aptoslabs.com/v1/spec#/), using which a client can interact with the Aptos blockchain, is not designed to support data shaping. Moreover, the ledger data you get back using this API contains the data only for the transactions **initiated by you**. It does not provide the data for the transactions initiated by the others. This data is insufficient and too slow for an application that must access the blockchain data in an omniscient way to serve multiple users of the application. 

Indexer is a solution to this problem. See below a high-level block diagram of how Aptos indexing works. 

<center>
<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/aptos-indexing.svg'),
    dark: useBaseUrl('/img/docs/aptos-indexing-dark.svg'),
  }}
/>
</center>

## Indexing the Aptos blockchain data

Indexing on the Aptos blockchain works like this:

- Users of a dApp, for example, on an NFT marketplace, interact with the Aptos blockchain via a rich UI presented by the dApp. Behind the scenes, these interactions generate, via smart contracts, the transaction and event data. This raw data is stored in the distributed ledger database, for example, on an Aptos fullnode.
- This raw ledger data is read and indexed using an application-specific data model, in this case an NFT marketplace-specific data model (”Business logic” in the above diagram). This NFT marketplace-specific index is then stored in a separate database (”Indexed database” in the above diagram).
- The dApp sends NFT-specific GraphQL queries to this indexed database and receives rich data back, which is then served to the users.

## Options for Aptos indexing service

Aptos supports the following ways to index the Aptos blockchain. 

1. Use the Aptos-provided indexing service with GraphQL API. This API is rate-limited and is intended only for lightweight applications such as wallets. This option is not recommended for high-bandwidth applications. This indexing service supports the following modules:
    1. **Token**: Only tokens that implement the Aptos `0x3::token::Token` standard. This indexer will only support 0x3 operations such as mint, create, transfer, offer, claim, and coin token swap. Also see Coin and Token.
    2. **Coin**: Supports only `0x1::coin::CoinStore`. This indexer will index any coins that appear in Aptos `CoinStore` standard but such coins may not have value unless they implement `0x1::coin::CoinInfo`.
2. Run your own indexer-enabled Aptos fullnode. With this option, the indexer supports, in addition to the above coin and token modules, basic transactions, i.e., each write set, events and signatures. 
3. Lastly, you can define your own data model (”Business Logic” in the above diagram) and set up the database for the index. 

A detailed documentation for each option is presented below.

## Use the Aptos-provided indexing service
Aptos offers a rate-limited graphql API for public use. 
[Testnet Link Here](https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql)
### Example token queries

Getting all tokens currently in account

```graphql
query CurrentTokens($owner_address: String, $offset: Int) {
  current_token_ownerships(
    where: {
      owner_address: {_eq: $owner_address},
      amount: {_gt: "0"},
      table_type: {_eq: "0x3::token::TokenStore"}
    }
    # Needed for pagination
    order_by: {last_transaction_version: desc}
    # Optional for pagination
    offset: $offset
  ) {
    token_data_id_hash
    name
    collection_name
    property_version
    amount
  }
}

# Query Variables
{
  "owner_address: "0xaa921481e07b82a26dbd5d3bc472b9ad82d3e5bfd248bacac160eac51687c2ff",
  "offset": 0
}
```

Getting all token activities for a particular token (note to get the token_id_hash you have to first make a query to get the token, e.g. from the first query).

```graphql
query TokenActivities($token_id_hash: String, $offset: Int) {
  token_activities(
    where: {token_data_id_hash: {_eq: $token_id_hash}}
    # Needed for pagination
    order_by: {transaction_version: desc}
    # Optional for pagination
    offset: $offset
  ) {
    transaction_version
    from_address
    property_version
    to_address
    token_amount
    transfer_type
  }
}

# Query Variables
{
  "token_id_hash": "f344b838264bf9aa57d5d4c1e0c8e6bbdc93f000abe3e7f050c2a0f4dc23d030",
  "offset": 0
}
```

Getting current token offered to account
```graphql
query CurrentOffers($to_address: String, $offset: Int) {
  current_token_pending_claims(
    where: {to_address: {_eq: $to_address}, amount: {_gt: "0"}}
    # Needed for pagination
    order_by: {last_transaction_version: desc}
    # Optional for pagination
    offset: $offset
  ) {
    token_data_id_hash
    name
    collection_name
    property_version
    from_address
    amount
  }
}

# Query Variables
{
  "to_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

### Example coin queries

Getting coin activities (including gas fees)

```graphql
query CoinActivity($owner_address: String, $offset: Int) {
  coin_activities(
    where: {owner_address: {_eq: $owner_address}}
    # Needed for pagination
    order_by: {transaction_version: desc}
    # Optional for pagination
    offset: $offset
  ) {
    activity_type
    amount
    coin_type
    entry_function_id_str
    transaction_version
  }
}

# Query Variables
{
  "owner_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

Currently owned coins (0x1::coin::CoinStore)

```graphql
query CurrentBalances($owner_address: String, $offset: Int)Ï {
  current_coin_balances(
    where: {owner_address: {_eq: $owner_address}}
    # Needed for pagination
    order_by: {last_transaction_version: desc}
    # Optional for pagination
    offset: $offset
  ) {
    owner_address    
    coin_type
    amount    
    last_transaction_timestamp
  }
}

# Query Variables
{
  "owner_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

### Example explorer queries

Getting all user transaction versions (to filter on user transaction for block explorer)

```graphql
query UserTransactions($limit: Int) {
  user_transactions(limit: $limit, order_by: {version: desc}) {
    version
  }
}

# Query Variables
{
  "limit": 10
}

```
## Run an indexer-enabled fullnode

See [Indexer Fullnode](/nodes/indexer-fullnode).

## Define your own data model

Currently Aptos only supports core modules such as 0x1::coin, 0x3::token, and 0x3::token_transfers. For other contracts, you’d likely need to implement custom parsing logic. 

High level, creating a custom indexer involves 4 steps: 

1. Define new table schemas in diesel
2. Create new data models based on the new tables
3. Create a new processor (or optionally add to an existing processor)
4. Integrate the new processor (optional if reusing existing processor)

Let’s look through these in details. Specifically we will use coin balances as an example, which is part of `coin_processor`. 

### 1. Define new table schemas in diesel

We use postgres and [diesel](https://diesel.rs/) as the ORM. To make sure that we make backward compatible changes without having to reset the database every upgrade, we use [diesel migrations](https://docs.diesel.rs/diesel_migrations/index.html) to manage the schema. This is why it’s very important to start with generating a new diesel migration before doing anything else. 

1. Create a new diesel migration. This will generate a new folder under [migrations](https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer/migrations) with `up.sql` and `down.sql`

```bash
DATABASE_URL=postgres://postgres@localhost:5432/postgres diesel migration generate add_coin_tables
```

b. Create the necessary table schemas. This is just postgres sql code. `up.sql` should have the new changes and `down.sql` should revert those changes.

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

[full up.sql and down.sql](https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer/migrations/2022-10-04-073529_add_coin_tables)

c. Run the migration. We suggest running it multiple times with `redo` to ensure that both `up.sql` and `down.sql` are implemented correctly. This will also modify the `[schema.rs](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/schema.rs)` file. 

```bash
DATABASE_URL=postgres://postgres@localhost:5432/postgres diesel migration run
DATABASE_URL=postgres://postgres@localhost:5432/postgres diesel migration redo
```

### 2. Create new data schemas
We now have to make rust data models that corresponds to the diesel schemas. In the case of coin balances, we will define `CoinBalance` and `CurrentCoinBalance`. 
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
We will also need to specify parsing logic, where the input is a portion of the transaction. In the case of coin balances, we can find all the details in `WriteSetChanges`, specifically where the write set change type is `write_resources`.

**Where can we find the relevant data for parsing?**
This requires a combination of understanding the move module and the structure of the transaction. In the example of coin balance, the contract lives in [coin.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move), specifically coin struct (search for `struct Coin`) that has a `value` field. We then can look at an [example transaction](https://fullnode.testnet.aptoslabs.com/v1/transactions/by_version/259518) where we find this exact structure in `write_resources`:
```
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

Find the full code in [coin_balances.rs](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/models/coin_models/coin_balances.rs).

### 3. Create a new processor
Now that we have the data model and the parsing function, we need to call that function and save the resulting model in our postgres database. The way we do this is by creating (or modifying) a `processor`. We've abstracted a lot already from that class, so the only function that need to be implemented is `process_transactions` (there are a few more functions that need to be copied, those should be obvious from the example). 

`process_transactions` takes in a vector of transactions with a start and end version that are used for tracking purposes. The general flow should be: 
   * Loop through transactions in the vector
   * Aggregate relevant models (sometimes deduping is required, e.g. in the case of `CurrentCoinBalance`)
   * Insert models into database in a single diesel transaction (this is important to ensure that we don't have partial writes)
   * Return status (error or success)

Checkout [coin_process.rs](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/processors/coin_processor.rs) for a relatively straightforward example. You can search for `coin_balances` in the page for the specific code snippet related to coin balances. 

**How do we decide whether to create a new processor**
This is completely up to you. The benefit of creating a new processor is that you're starting from scratch so you'll have full control over exactly what gets written to the db. The downside is that you'll have to maintain a new fullnode (since there's a 1:1 mapping between fullnode and processor). 

### 4. Integrate the new processor

This is the easiest step and involves just a few boiler plate additions. 

1. To start, make sure to add the new processor in [mod.rs](http://mod.rs) and [runtime.rs](http://runtime.rs)

[mod.rs](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/processors/mod.rs)

```rust
pub enum Processor {
  CoinProcessor,
  ...
}
...
  COIN_PROCESSOR_NAME => Self::CoinProcessor,
```

[runtime.rs](https://github.com/aptos-labs/aptos-core/blob/main/crates/indexer/src/runtime.rs)

```rust
Processor::CoinProcessor => Arc::new(CoinTransactionProcessor::new(conn_pool.clone())),
```

b. Create a fullnode.yaml with the correct config and test

I created a new fullnode_coin.yaml

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

Testing with command below (there are lots of logs so it’s helpful to filter to only indexer logs)

`cargo run -p aptos-node --features "indexer" --release -- -f ./fullnode_coin.yaml | grep -E "_processor"`
