---
title: "Shutting Down Nodes"
slug: "shutting-down-nodes"
---

# Shutting Down Nodes

Follow these instructions to shut down the validator node and validator fullnode, and cleanup the resources used by the nodes.

## Leaving the validator set

Before you shutdown the node, make sure to leave the validator set first. This will be become effective in the next epoch. Also note that a node can choose to leave the validator set at anytime, or it would happen automatically when there is insufficient stake in the validator account. To leave the validator set, run the below command, shown using the example profile of `mainnet-operator`:

```bash
aptos node leave-validator-set --profile mainnet-operator --pool-address <owner-address>
```

:::danger Important
If you leave and then rejoin in the same epoch, the rejoin would fail. This is because  when you leave, your validator state changes from "active" to "pending_inactive" but not yet "inactive". Hence the rejoin would fail.
::: 

After leaving the validator set, follow any one of the below sections to shut down your nodes. 

## Using source code

1. Stop your node.
2. Remove the data directory: `rm -r <your-data-directory>`.
3. Remove the genesis blob file and waypoint file.
4. If you want to reuse your node identity, you can choose to keep these configuration files: 
   - `private-keys.yaml`
   - `validator-identity.yaml`
   - `validator-full-node-identity.yaml` 
  
  or else you can delete these files.

## Using Docker

1. Stop your node and remove the data volumes: `docker compose down --volumes`.
2. Remove the genesis blob file and waypoint file.
3. If you want to reuse your node identity, you can choose to keep these configuration files: 
   - `private-keys.yaml`
   - `validator-identity.yaml`
   - `validator-full-node-identity.yaml` 
  
  or else you can delete these files.

## Using Terraform

- Stop your node and delete all the resources: `terraform destroy`.

