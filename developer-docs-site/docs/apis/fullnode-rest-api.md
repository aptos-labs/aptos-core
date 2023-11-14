---
title: "Fullnode Rest API"
slug: "fullnode-rest-api"
---

# Use the Aptos Fullnode REST API

If you with to employ the [Aptos API](https://aptos.dev/nodes/aptos-api-spec/#/), then this guide is for you. This guide will walk you through all you need to integrate the Aptos blockchain into your platform with the Aptos  API.

:::tip
Also see the [System Integrators Guide](../guides/system-integrators-guide.md) for a thorough walkthrough of Aptos integration.
:::

## Understanding rate limits

As with the [Aptos Indexer](../indexer/api/labs-hosted.md#rate-limits), the Aptos REST API has a rate limit of 5000 requests per five minutes by IP address, whether submitting transactions or querying the API on Aptos-provided nodes. (As a node operator, you may raise those limits on your own node.) Note that this limit can change with or without prior notice.

## Viewing current and historical state

Most integrations into the Aptos blockchain benefit from a holistic and comprehensive overview of the current and historical state of the blockchain. Aptos provides historical transactions, state, and events, all the result of transaction execution.

* Historical transactions specify the execution status, output, and tie to related events. Each transaction has a unique version number associated with it that dictates its global sequential ordering in the history of the blockchain ledger.
* The state is the representation of all transaction outputs up to a specific version. In other words, a state version is the accumulation of all transactions inclusive of that transaction version.
* As transactions execute, they may emit events. [Events](../concepts/events.md) are hints about changes in on-chain data.

:::important
Ensure the [fullnode](../nodes/networks.md) you are communicating with is up to date. The fullnode must reach the version containing your transaction to retrieve relevant data from it. There can be latency from the fullnodes retrieving state from [validator fullnodes](../concepts/fullnodes.md), which in turn rely upon [validator nodes](../concepts/validator-nodes.md) as the source of truth.
:::

The storage service on a node employs two forms of pruning that erase data from nodes:

* state
* events, transactions, and everything else

While either of these may be disabled, storing the state versions is not particularly sustainable.

Events and transactions pruning can be disabled via setting the [`enable_ledger_pruner`](https://github.com/aptos-labs/aptos-core/blob/cf0bc2e4031a843cdc0c04e70b3f7cd92666afcf/config/src/config/storage_config.rs#L141) to `false` in `storage_config.rs`. This is default behavior in Mainnet. In the near future, Aptos will provide indexers that mitigate the need to directly query from a node.

The REST API offers querying transactions and events in these ways:

* [Transactions for an account](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_account_transactions)
* [Transactions by version](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_transaction_by_version)
* [Events by event handle](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_events_by_event_handle)

## Reading state with the View function

View functions do not modify blockchain state when called from the API. A [View](https://github.com/aptos-labs/aptos-core/blob/main/api/src/view_function.rs) function and its [input](https://github.com/aptos-labs/aptos-core/blob/main/api/types/src/view.rs) can be used to read potentially complex on-chain state using Move. For example, you can evaluate who has the highest bid in an auction contract. Here are related files:

* [`view_function.rs`](https://github.com/aptos-labs/aptos-core/blob/main/api/src/tests/view_function.rs) for an example
* related [Move](https://github.com/aptos-labs/aptos-core/blob/90c33dc7a18662839cd50f3b70baece0e2dbfc71/aptos-move/framework/aptos-framework/sources/coin.move#L226) code
* [specification](https://github.com/aptos-labs/aptos-core/blob/90c33dc7a18662839cd50f3b70baece0e2dbfc71/api/doc/spec.yaml#L8513).

The view function operates like the [Aptos Simulation API](../guides/system-integrators-guide.md#testing-transactions-or-transaction-pre-execution), though with no side effects and a accessible output path. View functions can be called via the `/view` endpoint. Calls to view functions require the module and function names along with input type parameters and values.

A function does not have to be immutable to be tagged as `#[view]`, but if the function is mutable it will not result in state mutation when called from the API.
If you want to tag a mutable function as `#[view]`, consider making it private so that it cannot be maliciously called during runtime.

In order to use the View functions, you need to [publish the module](../move/move-on-aptos/cli.md#publishing-a-move-package-with-a-named-address) through the [Aptos CLI](../tools/aptos-cli/install-cli/index.md).

In the Aptos CLI, a view function request would look like this:
```
aptos move view --function-id devnet::message::get_message --profile devnet --args address:devnet
{
  "Result": [
    "View functions rock!"
  ]
}
```

In the TypeScript SDK, a view function request would look like this:
```
    const payload: Gen.ViewRequest = {
      function: "0x1::coin::balance",
      type_arguments: ["0x1::aptos_coin::AptosCoin"],
      arguments: [alice.address().hex()],
    };

    const balance = await client.view(payload);

    expect(balance[0]).toBe("100000000");
```

The view function returns a list of values as a vector. By default, the results are returned in JSON format; however, they can be optionally returned in Binary Canonical Serialization (BCS) encoded format.
