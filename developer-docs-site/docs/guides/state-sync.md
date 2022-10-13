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

:::tip Fastest and cheapest method
This is the fastest and cheapest method of syncing your node. It
requires the node to start from an empty state (i.e., not have any existing
storage data).
:::

:::caution Proceed with caution
Fast sync should only be used as a last resort for validators and
validator fullnodes. This is because fast sync skips all of the blockchain
history and as a result: (i) reduces the data availability in the network;
and (ii) may hinder validator consensus performance if too much data has
been skipped. Thus, validator and validator fullnode operators should be
careful to consider alternate ways of syncing before resorting to fast sync.
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

**Note:** If `aptos_state_sync_version{type="synced_states"}` does not 
increase then do the following:
1. Double-check the node configuration file has correctly been updated.
2. Make sure that the node is starting up with an empty storage database
(i.e., that it has not synced any state previously).
3. Add the following to your node configuration to account for any potential
network delays that may occur when initializing slow network connections:

```yaml
 state_sync:
   state_sync_driver:
     ...
     max_connection_deadline_secs: 1000000 # Tolerate slow peer discovery & connections
```

## Security implications and data integrity
Each of the different syncing modes perform data integrity verifications to
ensure that the data being synced to the node has been correctly produced
and signed by the validators. This occurs slightly differently for
each syncing mode:
1. Executing transactions from genesis is the most secure syncing mode. It will
verify that all transactions since the beginning of time were correctly agreed
upon by consensus and that all transactions were correctly executed by the
validators. All resulting blockchain state will thus be re-verified by the
syncing node.
2. Applying transaction outputs from genesis is faster than executing all
transactions, but it requires that the syncing node trusts the validators to
have executed the transactions correctly. However, all other
blockchain state is still manually re-verified, e.g., consensus messages,
the transaction history and the state hashes are still verified.
3. Fast syncing skips the transaction history and downloads the latest
blockchain state before continuously syncing. To do this, it requires that the
syncing node trust the validators to have correctly agreed upon all
transactions in the transaction history as well as trust that all transactions
were correctly executed by the validators. However, all other blockchain state
is still manually re-verified, e.g., epoch changes and the resulting blockchain states.

All of the syncing modes get their root of trust from the validator set
and cryptographic signatures from those validators over the blockchain data.
For more information about how this works, see the [state synchronization blogpost](https://medium.com/aptoslabs/the-evolution-of-state-sync-the-path-to-100k-transactions-per-second-with-sub-second-latency-at-52e25a2c6f10).

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
