---
title: "State Synchronization"
slug: "state-sync"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# State Synchronization

Nodes in an Aptos network (e.g., validator nodes and fullnodes) must always be synchronized to the latest Aptos blockchain state. The [state synchronization](https://medium.com/aptoslabs/the-evolution-of-state-sync-the-path-to-100k-transactions-per-second-with-sub-second-latency-at-52e25a2c6f10) (state sync) component that runs on each node is responsible for this. State sync identifies and fetches new blockchain data from the peers, validates the data and persists it to the local storage.

:::tip Need to start a node quickly?
If you need to start a node quickly, here's what we recommend by use case:
  - **Devnet public fullnode**: To sync the entire blockchain history, use [output syncing](state-sync.md#applying-all-transaction-outputs). Otherwise, use [fast sync](state-sync.md#fast-syncing).
  - **Testnet public fullnode**: To sync the entire blockchain history, restore from a [backup](../nodes/full-node/aptos-db-restore.md). Otherwise, download [a snapshot](../nodes/full-node/bootstrap-fullnode.md) or use [fast sync](state-sync.md#fast-syncing).
  - **Mainnet public fullnode**: To sync the entire blockchain history, restore from a [backup](../nodes/full-node/aptos-db-restore.md). Otherwise, use [fast sync](state-sync.md#fast-syncing).
  - **Mainnet validator or validator fullnode**: To sync the entire blockchain history, restore from a [backup](../nodes/full-node/aptos-db-restore.md). Otherwise, use [fast sync](state-sync.md#fast-syncing).
:::

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
[`aptos_state_sync_version{type="synced"}`](../nodes/full-node/fullnode-source-code-or-docker.md#verify-initial-synchronization) metric gradually increase.
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
[`aptos_state_sync_version{type="synced"}`](../nodes/full-node/fullnode-source-code-or-docker.md#verify-initial-synchronization) metric gradually increase.
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

## Running archival nodes

To operate an archival node, which is a fullnode that contains all blockchain data
since the start of the blockchain's history (that is, genesis), you should:
1. Run a fullnode and configure it to execute all transactions, or apply all transaction outputs (see above).
Do not select fast syncing, as the fullnode will not contain all data since genesis.
2. Disable the ledger pruner, as described in the [Data Pruning document](data-pruning.md#disabling-the-ledger-pruner).
This will ensure that no data is pruned and the fullnode contains all blockchain data.

:::caution Proceed with caution
Running and maintaining archival nodes is likely to be expensive and slow
as the amount of data being stored on the fullnode will continuously grow.
:::


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
