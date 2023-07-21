---
title: "Update Aptos Validator Node"
slug: "update-validator-node"
---

# Update Aptos Validator Node via Failover

You will likely have to upgrade or replace your validator node (VN) at some point, such as for maintenance or outages. Start anew by [creating a new validator fullnode (VFN)](running-validator-node/index.md). To minimize downtime, we recommend you then convert your live validator fullnode to your validator node, and backfill the validator fullnode.

Since you are already running [a validator node and a validator fullnode](node-requirements.md), you have at your fingertips the means to replace your validator node immediately. Simply convert your validator fullnode to a validator node and then backfill the validator fullnode with either the updated validator node or an entirely new validator fullnode.

This page explains how to make this swap, which largely amounts to switching out files and configuration settings between the two nodes. For a community-provided version of this document for Docker setup, see [Failover and migrate Validator Nodes for less downtime](https://forum.aptoslabs.com/t/failover-and-migrate-validator-nodes-for-less-downtime/144846).

## Prepare

First, understand the data is almost identical between the two nodes. The VFN is missing the `consensus_db` and `secure-data.json`, but it is otherwise largely ready for conversion into a validator node.

To failover from an outdated or erroneous validator node to an updated and reliable validator fullnode, follow these steps:

1. Ensure your machine meets the [validator hardware requirements](node-requirements.md#hardware-requirements).
1. Update your validator fullnode with the latest version of the [Aptos CLI](../../../tools/aptos-cli/install-cli/index.md)
1. Copy the configuration files between the two nodes. See the files in the [validator setup](running-validator-node/index.md) documentation you used for the full list.
1. Synchonize data on the validator fullnode:
   * For mainnet, use [state synchronization](../../../guides/state-sync.md).
   * For devnet or testnet, [bootstrap a new fullnode from snapshot](../../full-node/bootstrap-fullnode.md).

## Configure

Remember to take the normal measures to connect your node to the Aptos network and establish staking pool operations, such as removing the `secure-data.json` file and updating your `account_address` in the `validator-identity.yaml` and `validator-fullnode-identity.yaml` files to your **pool** address.

See the sections and guides below for full details.

### Connect to Aptos network

After deploying your nodes, [connect to the Aptos Network](./connect-to-aptos-network.md).

### Set up staking pool operations

After connecting your nodes to the Aptos network, [establish staking pool operations](./staking-pool-operations.md).

## Failover

To replace the validator node:

1. Update DNS to [swap the node network addresses on-chain](./staking-pool-operations.md#3-update-validator-network-addresses-on-chain).
1. Turn down the validator node and validator fullnode intended to replace the validator.
1. Restart the former validator fullnode with the validator node configuration.
1. Observe that before DNS changes take effect that only outbound connections will form.
1. Either reuse the former validator node or create anew to backfill the validator fullnode.
1. Start the validator fullnode.
1. Use [Node Health Checker](../../measure/node-health-checker.md) and follow [Node Liveness Criteria](node-liveness-criteria.md) to ensure the validator node is functioning properly.

## Run multiple validator fullnodes

You may want to have a VFN ready for failover or need access to REST APIs for building without any rate limits. Note you have the ability to run a [local multinode network](../../../guides/running-a-local-multi-node-network.md) that may be suitable.

With caution, you may also run multiple fullnodes on the Aptos network. Note that it is not currently recommended to run multiple VFNs with the same [network identity](../../identity-and-configuration.md) and connect them to the validator using the `vfn` network, as this may cause issues with node metrics and telemetry.

To run multiple fullnodes and connect them to your validator:

1. Connect only one fullnode using the `vfn` network configuration in the validator configuration `.yaml` file. This will be your single VFN (as registered on-chain) that other Aptos nodes will connect to.
1. Connect the rest of your fullnodes to the validator using a `public` network configuration *and a different network identity* in the validator configuration `.yaml` file. These will be your additional VFNs that you can use for other purposes.

Note that because the additional VFNs will not be registered on-chain, other nodes will not know their network addresses and will not be able to to connect to them. These would be for your use only.
