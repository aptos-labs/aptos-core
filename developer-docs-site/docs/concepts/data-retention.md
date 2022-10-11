---
title: "Data Retention"
slug: "data-retention"
---

# Data Retention

While a Aptos Node is running, it participates in the consensus to generate new data to the ledger or sync the new data from other nodes via [State Sync](/concepts/state-sync).

With the chain growing fast, old ledger data needs to be pruned to cap the disk space occupation. The node has a default prune window builtin, but that can be overridden by editing the node configuration (for example, `fullnode.yaml` or `validator.yaml`).

## To disable the ledger pruner

Add these to the configuration:

```
storage:
 storage_pruner_config:
  ledger_pruner_config:
   enable: false
```

:::caution
The ledger size can grow very fast, you risk filling the disk up by disabling the ledger pruner.
:::

The complete set of storage configuration can be found in the [Storage README](https://github.com/aptos-labs/aptos-core/tree/main/storage#configs)