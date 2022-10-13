---
title: "Data Pruning"
slug: "data-pruning"
---

# Data Pruning

When a validator node is running, it participates in the consensus to generate new data to the ledger, or sync the new data from the other nodes via [State Sync](/guides/state-sync). With the ledger growing fast, storage disk space can be managed by pruning the old ledger data. By default pruning is enabled on the node, with a default pruning window. This document describes how you can change these settings. 

To manage these settings, edit the node configuration YAML files, for example, `fullnode.yaml` for fullnode (validator or public) or `validator.yaml` for validator node, as shown below.

## Disabling the ledger pruner

Add the below settings to the node configuration YAML file:

:::caution Proceed with caution
Disabling the pruning can result in the storage disk filling up quickly.
:::

```yaml
storage:
 storage_pruner_config:
  ledger_pruner_config:
   enable: false
```

## Changing the ledger prune window

Add the below settings to the node configuration YAML file to make the node retain, for example, 1 billion transactions and their outputs, including events and write sets.

:::caution Proceed with caution
Setting the prune window smaller than 100 million can lead to runtime errors and can damage the health of the network.
:::

```yaml
storage:
 storage_pruner_config:
  ledger_pruner_config:
    prune_window: 1000000000
```

See the complete set of storage configuration settings in the [Storage README](https://github.com/aptos-labs/aptos-core/tree/main/storage#configs).