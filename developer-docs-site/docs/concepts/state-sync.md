---
title: "State Synchronization"
slug: "state-sync"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# State Synchronization

Nodes in an Aptos network, both the validator nodes and the fullnodes, must always be synchronized to the latest Aptos blockchain state. The state synchronization (state sync) component that runs on each node is responsible for this synchronization. To achieve this synchronization, state sync identifies and fetches new blockchain data from the peers, validates the data and persists it to the local storage.

## State sync modes

State sync runs in two modes. All nodes will first bootstrap (in bootstrapping mode) on startup, and then continuously synchronize (in continuous sync mode). 

### Bootstrapping mode

When the node starts, state sync will perform bootstrapping by using the specified bootstrapping mode configuration. This allows the node to catch up to the Aptos blockchain. There are three bootstrapping modes:

- **Execute all the transactions since genesis**. In this state sync mode the node will retrieve from the Aptos network all the transactions since genesis, i.e., since the start of the blockchain's history, and re-execute those transactions. Naturally, this synchronization mode takes the longest amount of time.
- **Apply transaction outputs since genesis**. In this state sync mode the node will retrieve all the transactions since genesis but it will skip the transaction execution and will only apply the outputs of the transactions that were previously produced by validator execution. This mode reduces the amount of CPU time required.
- **Download the latest state directly**. In this state sync mode the node will skip the transaction history in the blockchain and will download only the latest blockchain state directly. As a result, the node will not have the historical transaction data, but it will be able to catch up to the Aptos network much more rapidly.

### Continuous syncing mode

After the node has bootstrapped and caught up to the Aptos network initially, state sync will then move into continuous syncing mode to stay up-to-date with the blockchain. There are two continuous syncing modes:

- **Executing transactions**. This state sync mode will keep the node up-to-date by executing new transactions as they are committed to the blockchain.
- **Applying transaction outputs**. This state sync mode will keep the node up-to-date by skipping the transaction execution and only applying the outputs of the transactions as previously produced by validator execution.

## Configuring the state sync modes

The below sections provide instructions for how to configure your node for different use cases.

### Executing all transactions

To execute all the transactions since genesis and continue to execute new
transactions as they are committed, add the following to your node
configuration file (for example,`fullnode.yaml` or `validator.yaml`):

```yaml
 state_sync:
     state_sync_driver:
         bootstrapping_mode: ExecuteTransactionsFromGenesis
         continuous_syncing_mode: ExecuteTransactions
```

:::tip Verify node syncing
While your node is syncing, you'll be able to see the
[`aptos_state_sync_version{type="synced"}`](/nodes/full-node/fullnode-source-code-or-docker/#verify-initial-synchronization) metric gradually increase.
:::

### Applying all transaction outputs

To apply all transaction outputs since genesis and continue to apply new
transaction outputs as transactions are committed, add the following to your
node configuration file:

```yaml
 state_sync:
     state_sync_driver:
         bootstrapping_mode: ApplyTransactionOutputsFromGenesis
         continuous_syncing_mode: ApplyTransactionOutputs
```

:::tip Verify node syncing
While your node is syncing, you'll be able to see the
[`aptos_state_sync_version{type="synced"}`](/nodes/full-node/fullnode-source-code-or-docker/#verify-initial-synchronization) metric gradually increase.
:::

## Fast syncing

:::tip 
This is the fastest and cheapest method of syncing your node.
:::

To download the latest blockchain state and continue to apply new
transaction outputs as transactions are committed, add the following to your
node configuration file:

```yaml
 state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
```

While your node is syncing, you'll be able to see the
`aptos_state_sync_version{type="synced_states"}` metric gradually increase.
However, `aptos_state_sync_version{type="synced"}` will only increase once
the node has bootstrapped. This may take several hours depending on the 
amount of data, network bandwidth and node resources available.

## State sync architecture

The state synchronization component is comprised of four sub-components, each with a specific purpose:

1. **Driver**: The driver “drives” the synchronization progress of the node.
It is responsible for verifying all data that the node receives from peers. Data
is forwarded from the peers via the data streaming service. After data
verification, the driver persists the data to storage.
2. **Data Streaming Service**: The streaming service creates data streams for
clients (one of which is the state sync driver). It allows the client to stream
new data chunks from peers, without having to worry about which peers have the
data or how to manage data requests. For example, the client can request all
transactions since version `5` and the data streaming service will provide
this.
3. **Aptos Data Client**: The data client is responsible for handling data
requests from the data streaming service. For the data streaming service to
stream all transactions, it must make multiple requests (each request for a
batch of transactions) and send those requests to peers (e.g., transactions
`1→5`, `6→10`, `11→15`, and so on). The data client takes the request,
identifies which peer can handle the request and sends the request to them.
4. **Storage Service**: The storage service is a simple storage API offered by
each node which allows peers to fetch data. For example, the data client on
peer `X` can send the data request to the storage service on peer `Y` to fetch
a batch of transactions.

## State sync code structure

Below are the links to the state sync code showing the structure that matches the architecture outlined above:
- **Driver:** [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/state-sync-driver](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/state-sync-driver)
- **Data Streaming Service:** [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/data-streaming-service](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/data-streaming-service)
- **Aptos Data Client**: [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/aptos-data-client](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/aptos-data-client)
- **Storage Service:** [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/storage-service](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/storage-service)

In addition, see also a directory containing the code for
**inter-component** communication: [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/inter-component](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/inter-component).
This is required so that:
   - State sync can handle notifications from consensus (e.g., to catch up after falling behind).
   - State sync can notify mempool when transactions are committed (i.e., so they can be removed from mempool).
   - State sync can update the event subscription service to notify listeners (e.g., other system components for reconfiguration events).
