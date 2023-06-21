---
title: "Use the Aptos Indexer"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Use the Aptos Indexer

This page describes how to employ data from the Aptos Indexer in your apps. To instead operate an indexer, follow [Run an Indexer](../nodes/indexer-fullnode.md).

Typical applications built on the Aptos blockchain, on any blockchain for that matter, require the raw blockchain data to be shaped and stored in an application-specific manner. This is essential to supporting low-latency and rich experiences when consuming blockchain data in end-user apps from millions of users. The [Aptos Node API](https://aptos.dev/nodes/aptos-api-spec#/) provides a lower level, stable and generic API and is not designed to support data shaping or therefore such rich end-user experiences directly.

The Aptos Indexer is the answer to this need, allowing the data shaping critical to real-time app use. See this high-level diagram for how Aptos indexing works:

<center>
<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/aptos-indexing.svg'),
    dark: useBaseUrl('/img/docs/aptos-indexing-dark.svg'),
  }}
/>
</center>

Indexing on the Aptos blockchain works like this:

- Users of a dApp, for example, on an NFT marketplace, interact with the Aptos blockchain via a rich UI presented by the dApp. Behind the scenes, these interactions generate, via smart contracts, the transaction and event data. This raw data is stored in the distributed ledger database, for example, on an Aptos fullnode.
- This raw ledger data is read and indexed using an application-specific data model, in this case an NFT marketplace-specific data model (”Business logic” in the above diagram). This NFT marketplace-specific index is then stored in a separate database (”Indexed database” in the above diagram).
- The dApp sends NFT-specific GraphQL queries to this indexed database and receives rich data back, which is then served to the users.

## Options for Aptos indexing service

Aptos supports the following ways to index the Aptos blockchain. 

1. Use the Aptos Labs hosted indexing service with GraphQL API. This API is rate-limited and is intended only for lightweight applications such as wallets. This option is not recommended for high-bandwidth applications. This indexing service supports the following modules:
    1. **Token**: Only tokens that implement the Aptos `0x3::token::Token` standard. This indexer will only support 0x3 operations such as mint, create, transfer, offer, claim, and coin token swap. Also see Coin and Token.
    2. **Coin**: Supports only `0x1::coin::CoinStore`. This indexer will index any coins that appear in Aptos `CoinStore` standard but such coins may not have value unless they implement `0x1::coin::CoinInfo`.
2. Run your own indexer-enabled Aptos fullnode. With this option, the indexer supports, in addition to the above coin and token modules, basic transactions, i.e., each write set, events and signatures. 
3. Lastly, you can define your own data model (”Business Logic” in the above diagram) and set up the database for the index. 

A detailed documentation for each option is presented below.

## Use the Aptos-provided indexing service

Aptos provides a rate-limited GraphQL API for public use. See below a few examples showing how to use it.

### Aptos indexer GraphQL servers

- **Mainnet:** https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql
- **Testnet:** https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql
- **Devnet:** https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql

### Format of the address

Make sure that the address (either owner address or any Aptos blockchain account address) in the query contains a prefix of `0x` followed by the 64 hex characters, for example, `0xaa921481e07b82a26dbd5d3bc472b9ad82d3e5bfd248bacac160eac51687c2ff`.

### Running example queries

- Click on [Mainnet GraphQL server](https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql) or [Testnet GraphQL server](https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql) or [Devnet GraphQL server](https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql).
- On the server page, paste the **Query** code from an example into the main query section, and the **Query variables** code from the same example into the QUERY VARIABLES section (below the main query section).



### Example token queries

Getting all tokens currently in account. 

**Query**

```graphql
query CurrentTokens($owner_address: String, $offset: Int) {
  current_token_ownerships(
    where: {owner_address: {_eq: $owner_address}, amount: {_gt: "0"}, table_type: {_eq: "0x3::token::TokenStore"}}
    order_by: {last_transaction_version: desc}
    offset: $offset
  ) {
    token_data_id_hash
    name
    collection_name
    property_version
    amount
  }
}
```

**Query variables**
```json
{
  "owner_address": "0xaa921481e07b82a26dbd5d3bc472b9ad82d3e5bfd248bacac160eac51687c2ff",
  "offset": 0
}
```

---

Getting all token activities for a particular token. **Note** that to get the `token_id_hash` you have to first make a query to get the token from the above query.

**Query**

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
```

**Query variables**

```json
{
  "token_id_hash": "f344b838264bf9aa57d5d4c1e0c8e6bbdc93f000abe3e7f050c2a0f4dc23d030",
  "offset": 0
}
```

---

Getting current token offered to account.

**Query**

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
```

** Query variables**

```json
{
  "to_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

### Example coin queries

Getting coin activities (including gas fees).

**Query**

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
```

**Query variables**

```json
{
  "owner_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

---

Currently owned coins (`0x1::coin::CoinStore`).

**Query**

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
```

**Query variables**

```json
{
  "owner_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

### Example explorer queries

Getting all user transaction versions (to filter on user transaction for block explorer).

**Query**

```graphql
query UserTransactions($limit: Int) {
  user_transactions(limit: $limit, order_by: {version: desc}) {
    version
  }
}
```

**Query variables**

```json
{
  "limit": 10
}
```

### Rate limits

The following rate limit applies for this Aptos-provided indexing service:

- For a web application that calls this Aptos-provided indexer API directly from the client (for example, wallet or explorer), the rate limit is currently 5000 requests per five minutes by IP address. **Note that this limit can change with or without prior notice.** 

If you are running a backend (server-side) application and want to call the indexer programmatically then you should run an indexer-enabled fullnode. 

## Run an indexer-enabled fullnode

See [Indexer Fullnode](../nodes/indexer-fullnode.md).

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

See the full instructions on how to start an indexer-enabled fullnode in [Indexer Fullnode](/nodes/indexer-fullnode).
