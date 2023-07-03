---
title: "IndexerClient Class"
slug: "typescript-sdk-indexer-client-class"
---

The [IndexerClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/providers/indexer.ts) is responsible for handling the communication between the client-side application and the blockchain network. It uses the [Hasura framework](https://hasura.io/) to generate a set of [GraphQL queries](https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql) that can be used to retrieve data from the blockchain. The queries are optimized for performance and can retrieve data in real-time.

## Usage

To use the `IndexerClient` class, you will need to create an instance of `IndexerClient` and call the desired API method. The `IndexerClient` object will handle the HTTP requests and responses and return the result to your application.

## Configuration

Before using the `IndexerClient` class, you will need to configure it with the necessary parameters. These parameters may include the Hasura endpoint URL, custom configuration, and any other required settings. You can configure the `IndexerClient` class by passing in the necessary parameters when you initialize the client object.

## Initialization

To initialize the `IndexerClient` class, you will need to pass in the necessary configuration parameters. Here is an example:

```ts
import { IndexerClient } from "aptos";

const client = new IndexerClient("https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql");
```

## Make API fetch calls

To make an API call, you will need to call the appropriate method on the `IndexerClient` object. The method name and parameters will depend on the specific API you are using. Here is an example:

```ts
const accountNFTs = await client.getAccountNFTs("0x123");
```

In this example, we are using the `getAccountNFTs()` method to retrieve the NFT of an account with the address `0x123`.

## Use custom queries

The TypeScript SDK provides frequently used queries by different users and/or apps and makes sure the queries are well structured to retrive the current response.

With that being said, one can structure custom queries and use the SDK to query the Aptos Indexer API. For that, the SDK exports a `queryIndexer()` method that accepts a `GraphqlQuery` type argument. The `GraphqlQuery` type has a `query` field of type `string` and an optional `variable` field of an `object` type.

Here is the `GraphqlQuery` type definition.

```ts
type GraphqlQuery = {
  query: string;
  variables?: {};
};
```

To use the `queryIndexer()` method, one should pass the GraphQL query. For example:

```ts
const query: string = `query getAccountTokensCount($owner_address: String) {
  current_token_ownerships_aggregate(where: { owner_address: { _eq: $owner_address }, amount: { _gt: "0" } }) {
    aggregate {
      count
    }
  }
}`;
const variables = { owner_address: "0x123" };
const graphqlQuery = { query, variables };
const accountTokensCount = await client.queryIndexer(graphqlQuery);
```

:::tip
Be aware that it queries the network endpoint you passed in when initializing the `IndexerClient` class.
:::

## Generate queries

To generate an Indexer query that can be used within the SDK, we can write a GraphQL query (based on the [Indexer schema](https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql)) and use the SDK to generate a TypeScript query.

### Write an Indexer query

All Indexer queries, which are basically GraphQL queries, live under the `src/indexer/queries/` folder. In this folder, we create a `.graphql` file for each query we want the SDK to support. For example, a `.graphql` file with a GraphQL query can be:

```graphql
query getAccountTokensCount($owner_address: String) {
  current_token_ownerships_aggregate(where: { owner_address: { _eq: $owner_address }, amount: { _gt: "0" } }) {
    aggregate {
      count
    }
  }
}
```

### Generate TypeScript queries

Once we have created a `.graphql` file with a GraphQL query, we can generate TypeScript code based on that query so we can use it with the TypeScript SDK by running the following command:

```cmd
pnpm run indexer-codegen
```

That command runs the `graphql-codegen` command that generates code from the Indexer GraphQL schema based on the SDK configuration file.

### SDK GraphQL configuration file

The TypeScript SDK uses a configuration file for [@graphql-codegen](https://the-guild.dev/graphql/codegen), a code generation tool for GraphQL.

The SDK [configuration file](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/indexer/codegen.yml) defines how `@graphql-codegen` should generate TypeScript code from a GraphQL schema and queries. Let's break down some of the key elements of this file:

- `schema` – Specifies the location of the GraphQL schema file that `@graphql-codegen` should use for code generation. In this case, it is using the Aptos Indexer `mainnet` schema.
- `documents` - Specifies the location of the GraphQL operation files that `@graphql-codegen` should use for code generation. In this case, it is using the `src/indexer/queries/` location (as mentioned in the previous section) and includes all files with the `.graphql` extension.
- `generates` – Defines the output files that `@graphql-codegen` should generate based on the schema and operations. In this case, it is generating the types, operations and queries.
- `plugins` – Specifies the plugins that `@graphql-codegen` should use for code generation. In this case, it is using the `typescript` plugin to generate TypeScript typings from the GraphQL schema, `typescript-operations` plugin to generate TypeScript typings for GraphQL operations and `typescript-graphql-request` plugin to generate function for making GraphQL requests .

:::tip
You can use the `IndexerClient` class directly or the [Provider](./sdk-client-layer.md) class (preferred).
:::
