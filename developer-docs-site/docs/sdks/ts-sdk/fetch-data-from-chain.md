---
title: "Fetch data from chain"
---

Once we created a new [Aptos instance](./sdk-configuration.md), we get access to all the sdk functionality. We now can query the chain for data.

The SDK provides built in queries to easily query the chain with most used or popular queries. The SDK resolves those queries to Aptos [fullnode](https://fullnode.mainnet.aptoslabs.com/v1/spec#/) or [Indexer](https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql) as needed and ease the burden on the developer to know and understand what service they need to query.

```ts
const aptos = new Aptos();

const fund = await aptos.getAccountInfo({ accountAddress: "0x123" });
const modules = await aptos.getAccountTransactions({ accountAddress: "0x123" });
const tokens = await aptos.getAccountOwnedTokens({ accountAddress: "0x123" });
```

### Queries with generics

Some query responses do not provide the full response type as the SDK can't infer the actual type. For that we might want to provide a generic type for the response type, so we can access the response properties that are not included in the API type.

For example, for the `getAccountResource` query we can define the `resource` to query but the SDK can't infer the response type and we can't have access to the response properties.

For that we support generic response types for different queries.

```ts
type Coin = { coin: { value: string } };

const resource = await aptos.getAccountResource<Coin>({
  accountAddress: testAccount.accountAddress,
  resourceType: "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
});

// Now we have access to the response type property
const value = resource.coin.value;
```

### `options` input argument

We can provide queries with an `options` input as query parameters. For those queries that support this option, an `option` input param is available

```ts
const resource = await aptos.getAccountResource({
  accountAddress: alice.accountAddress,
  resourceType: "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
  options: { ledgerVersion: 12 },
});

const tokens = await aptos.getAccountOwnedTokens({
  accountAddress: alice.accountAddress,
  options: {
    tokenStandard: "v2",
    pagination: { offset: 0, limit: 10 },
    orderBy: [{ last_transaction_version: "desc" }],
  },
});
```

### Wait for Indexer to sync up

Sometimes we use Indexer service to fetch data, this is because we can not get complex data direct from fullnode or some queries are not supported with the fullnode API.
Since Indexer indexes the chain, it might take it some time to catch up with the latest ledger version and we can end up not getting the real time data.

For that, the SDK supports an optional input argument `minimumLedgerVersion`. We can pass a ledger version to sync up to, before querying.
If no version provided, the SDK will not wait for Indexer to sync up.

```ts
const tokens = await aptos.getAccountOwnedTokens({
  accountAddress: alice.accountAddress,
  minimumLedgerVersion: 1234,
});
```

To get the latest ledger version we can

1. Query for the ledger info

```ts
const ledgerInfo = await aptos.getLedgerInfo();

const ledgerVersion = ledgerInfo.ledger_version;
```

2. If we just commited a transaction with the SDK, we can use `waitForTransaction` method, that would return us a `CommittedTransactionResponse` that holds the latest ledger version

```ts
const response = await aptos.waitForTransaction({ transactionHash: pendingTransaction.hash });

const tokens = await aptos.getAccountOwnedTokens({
  accountAddress: alice.accountAddress,
  minimumLedgerVersion: BigInt(response.version),
});
```

### Use namespace

The `Aptos` class holds different namespaces related to the query operation we seek to do. For example, all `account` related queries are under the `aptos.account` namespace.
Once we intiate the `Aptos` class, all namespaces will be available for as with autocomplete along with all the possible API functions.

Thought we dont need to specify the namespace when making a query, it can be beneficial while developing.

```ts
const aptos = new Aptos()
aptos.< list of available API functions and namespaces >
```
