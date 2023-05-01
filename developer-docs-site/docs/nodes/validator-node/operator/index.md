---
title: "Operator"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Operator

If you are an operator participating in the Aptos network, then use this document to perform the operator tasks such as deploying a validator node and validator fullnode, registering the nodes on the Aptos community platform, and performing the validation. 

:::tip Both validator node and validator fullnode are required for mainnet
For participating in the Aptos mainnet, you must deploy both a validator node and a validator fullnode. 
:::

## Deploy the nodes and register

**Step 1:** Read the [**Node Requirements**](./node-requirements.md) and make sure that your hardware, storage and network resources satisfy the node requirements.

**Step 2:** **Deploy the nodes**. Follow the detailed node installation steps provided in [**Running Validator Node**](running-validator-node/index.md) and deploy a validator node and a validator fullnode.

Note that your nodes will not be running correctly (not syncing, not participating in consensus), until they're added to the validator set via [staking pool operations](./shutting-down-nodes.md) (below).

## Connect to Aptos network

After deploying your nodes, [connect to the Aptos Network](./connect-to-aptos-network.md).

## Set up staking and delegation pool operations

After connecting your nodes to the Aptos network, establish [staking pool operations](./staking-pool-operations.md) to add your node to the validator set. 

Similarly, conduct [delegation pool operations](./delegation-pool-operations.md) for APT delegated to your validator. Your node will start syncing and participating in consensus.

## Ensure your nodes are live

After your nodes are deployed and configure, make sure they meet [node liveness criteria](./node-liveness-criteria.md).
