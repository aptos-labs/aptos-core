---
title: "SDK Configuration"
---

# TS SDK Configuration

## `Aptos` class

The first step to interact with the Aptos chain using the SDK it to intansiate an `Aptos` class. This class is the main entry point into Aptos's APIs.

```ts
const aptos = new Aptos();
```

## `AptosConfig` class

Sometimes you might want to use custom configurations when interacting with the `Aptos` chain. For that we have `AptosConfig` class that holds the config information for the SDK client instance.

```ts
const aptosConfig = new AptosConfig({...})
```

## Available configuration

```ts
/** The Network that this SDK is associated with. Defaults to DEVNET */
readonly network: Network;

/**
 * The client instance the SDK uses. Defaults to `@aptos-labs/aptos-client`
 */
readonly client: Client;

/**
 * The optional hardcoded fullnode URL to send requests to instead of using the network
 */
readonly fullnode?: string;

/**
 * The optional hardcoded faucet URL to send requests to instead of using the network
 */
readonly faucet?: string;

/**
 * The optional hardcoded indexer URL to send requests to instead of using the network
 */
readonly indexer?: string;

/**
 * A configuration object we can pass with the request to the server.
 */
readonly clientConfig?: ClientConfig;

```
