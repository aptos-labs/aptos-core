---
title: "Example Queries"
---

# Example Indexer API Queries

## Running example queries

1. Open the Hasura Explorer for the network you want to query. You can find the URLs [here](/indexer/api/labs-hosted#hasura-explorer).
1. Paste the **Query** code from an example into the main query section, and the **Query Variables** code from the same example into the Query Variables section (below the main query section).

## More Examples
You can find many more example queries in the [TypeScript SDK](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk/src/indexer/queries). Indeed if you're using the TypeScript SDK, you should look at the [IndexerClient](../../sdks/ts-sdk/typescript-sdk-indexer-client-class).

## Example Token Queries

Getting all tokens currently in account.

**Query**

```graphql
query CurrentTokens($owner_address: String, $offset: Int) {
  current_token_ownerships(
    where: {owner_address: {_eq: $owner_address}, amount: {_gt: "0"}, table_type: {_eq: "0x3::token::TokenStore"}}
    order_by: [{last_transaction_version: desc}, {token_data_id: desc}]
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

**Query Variables**
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
    order_by: [{last_transaction_version: desc}, {event_index: asc}]
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

**Query Variables**

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
    order_by: [{last_transaction_version: desc}, {token_data_id: desc}]
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

** Query Variables**

```json
{
  "to_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

## Example Coin Queries

Getting coin activities (including gas fees).

**Query**

```graphql
query CoinActivity($owner_address: String, $offset: Int) {
  coin_activities(
    where: {owner_address: {_eq: $owner_address}}
    # Needed for pagination
    order_by: [{last_transaction_version: desc}, {event_index: asc}]
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

**Query Variables**

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
    order_by: [{last_transaction_version: desc}, {token_data_id: desc}]
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

**Query Variables**

```json
{
  "owner_address": "0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb",
  "offset": 0
}
```

## Example Explorer Queries

Getting all user transaction versions (to filter on user transaction for block explorer).

**Query**

```graphql
query UserTransactions($limit: Int) {
  user_transactions(limit: $limit, order_by: {version: desc}) {
    version
  }
}
```

**Query Variables**

```json
{
  "limit": 10
}
```
