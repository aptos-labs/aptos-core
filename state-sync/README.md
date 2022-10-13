---
id: state sync
title: State Sync
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/state-sync/README.md
---

# State Synchronization (State Sync)

State sync is a component that runs within each Aptos node and is responsible
for synchronizing the node to the latest blockchain state. It is required by
both validators and fullnodes to ensure that they do not fall behind the rest
of the network. To achieve this, state sync identifies and fetches new
blockchain data from peers, validates the data and persists it to local
storage.

## State sync modes

State sync can operate in different synchronization modes, depending on the
data the node operator would like to synchronize. There are two different
operations that users can configure when running their nodes:
1. **Bootstrapping mode**: The bootstrapping mode is the mode the node uses to 
get up-to-date. There are three possible bootstrapping modes:
   1. **Execute all transactions since genesis**. This will retrieve all
   transactions since genesis (i.e., the start of the blockchain's history) and
   re-execute those transactions. Naturally, this synchronization mode takes
   the longest amount of time.
   2. **Apply transaction outputs since genesis**. This will retrieve all
   transactions since genesis but it will skip transaction execution and only
   apply the outputs of the transactions as previously produced by validator
   execution. This reduces the amount of CPU time required.
   3. **Download the latest state directly**. This will skip the transaction
   history in the blockchain and download the latest blockchain state directly.
   As a result, the node won't have historical transaction data, but it will
   be able to catch up much more quickly.

2. **Continuous syncing mode**: The continuous syncing mode is the mode the
node uses to stay up-to-date once bootstrapped. There are two possible
continuous syncing modes:
   1. **Executing transactions**. This will keep the node up-to-date by
   executing new transactions as they are committed to the blockchain.
   2. **Applying transaction outputs**. This will keep the node up-to-date by
   skipping transaction execution and simply applying the outputs of the
   transactions as previously produced by validator execution.

The sections below provide instructions for how to configure your node for
various different use-cases.

### Executing all transactions

To execute all transactions since genesis and continue to execute new
transactions as they are committed, add the following to your node
configuration file:

```
 state_sync:
     state_sync_driver:
         bootstrapping_mode: ExecuteTransactionsFromGenesis
         continuous_syncing_mode: ExecuteTransactions
```

While your node is syncing, you'll be able to see the
`aptos_state_sync_version{type="synced"}` metric gradually increase.

### Applying all transaction outputs

To apply all transaction outputs since genesis and continue to apply new
transaction outputs as transactions are committed, add the following to your
node configuration file:

```
 state_sync:
     state_sync_driver:
         bootstrapping_mode: ApplyTransactionOutputsFromGenesis
         continuous_syncing_mode: ApplyTransactionOutputs
```

While your node is syncing, you'll be able to see the
`aptos_state_sync_version{type="synced"}` metric gradually increase.

### Fast Syncing
Note: Fast sync should only be used as a last resort for validators and
validator fullnodes. This is because fast sync skips all of the blockchain
history and as a result: (i) reduces the data availability in the network;
and (ii) may hinder validator consensus performance if too much data has
been skipped. Thus, validator and validator fullnode operators should be
careful to consider alternate ways of syncing before resorting to fast sync.

Note: this is the fastest and cheapest method of syncing your node. It
requires the node to start from an empty state (i.e., not have any existing
storage data).

To download the latest blockchain state and continue to apply new
transaction outputs as transactions are committed, add the following to your
node configuration file:

```
 state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
```

While your node is syncing, you'll be able to see the
`aptos_state_sync_version{type="synced_states"}` metric gradually increase.
However, `aptos_state_sync_version{type="synced"}` will only increase once
the node has boostrapped. This may take several hours depending on the 
amount of data, network bandwidth and node resources available.

**Note:** If `aptos_state_sync_version{type="synced_states"}` does not
increase then do the following:
1. Double-check the node configuration file has correctly been updated.
2. Make sure that the node is starting up with an empty storage database
   (i.e., that it has not synced any state previously).
3. Add the following to your node configuration to account for any potential
   network delays that may occur when initializing slow network connections:

```
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

State sync is broken down into 4 sub-components, each with a specific purpose:

1. **Driver**: The driver “drives” the synchronization progress of the node.
It is responsible for verifying all data that it receives from peers. Data
is forwarded from peers via the data streaming service. After data
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

### Code structure

The state sync code structure matches the architecture outlined above:
- **Driver:** [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/state-sync-driver](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/state-sync-driver)
- **Data Streaming Service:** [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/data-streaming-service](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/state-sync-v2/data-streaming-service)
- **Aptos Data Client**: [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/aptos-data-client](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/aptos-data-client)
- **Storage Service:** [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/storage-service](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/storage-service)

In addition, there is also a directory containing the code for
**inter-component** communication: [https://github.com/aptos-labs/aptos-core/tree/main/state-sync/inter-component](https://github.com/aptos-labs/aptos-core/tree/main/state-sync/inter-component).
This is required so that:
   - State sync can handle notifications from consensus (e.g., to catch up after falling behind)
   - State sync can notify mempool when transactions are committed (i.e., so they can be removed from mempool)
   - State sync can update the event subscription service to notify listeners (e.g., other system components for reconfiguration events)

