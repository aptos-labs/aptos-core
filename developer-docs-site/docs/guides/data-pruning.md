---
title: "Data Pruning"
slug: "data-pruning"
---

# Data Pruning

When a validator node is running, it participates in consensus to execute
transactions and commit new data to the blockchain. Similarly, when fullnodes
are running, they sync the new blockchain data through [state synchronization](../guides/state-sync.md).
As the blockchain grows, storage disk space can be managed by pruning old
blockchain data. Specifically, by pruning the **ledger history**: which
contains old transactions. By default, ledger pruning is enabled on all
nodes with a pruning window that can be configured. This document describes
how you can configure the pruning behavior.

:::note
By default the ledger pruner keeps 150 million recent transactions. The approximate amount of disk space required for every 150M transactions is 200G. Unless 
bootstrapped from the genesis and configured to disable the pruner or a long 
prune window, the node doesn't carry the entirety of the ledger history. 
Majority of the nodes on both the testnet and mainnet have a partial 
history of 150 million transactions according to this configuration.
:::


To manage these settings, edit the node configuration YAML files,
for example, `fullnode.yaml` for fullnodes (validator or public) or
`validator.yaml` for validator nodes, as shown below.

## Disabling the ledger pruner

Add the following to the node configuration YAML file to disable the
ledger pruner:

:::caution Proceed with caution
Disabling the ledger pruner can result in the storage disk filling up very quickly.
:::

```yaml
storage:
 storage_pruner_config:
  ledger_pruner_config:
   enable: false
```

## Configuring the ledger pruning window

Add the following to the node configuration YAML file to make the node
retain, for example, 1 billion transactions and their outputs, including events
and write sets.

:::caution Proceed with caution
Setting the pruning window smaller than 100 million can lead to runtime errors and damage the health of the node.
:::

```yaml
storage:
 storage_pruner_config:
  ledger_pruner_config:
    prune_window: 1000000000
```

See the complete set of storage configuration settings in the [Storage README](https://github.com/aptos-labs/aptos-core/tree/main/storage#configs).
