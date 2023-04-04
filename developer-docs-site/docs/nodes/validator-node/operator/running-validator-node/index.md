---
title: "Running Validator Node"
slug: "running-validator-node"
---

# Running Validator Node

:::tip Deploying a validator node? Read this first
If you are deploying a validator node, then make sure to read the [Node Requirements](../node-requirements.md) first.
:::

## Install Validator node

### Deploy

The following guides provide step-by-step instructions for running public fullnode, validator node, and validator fullnode for the Aptos blockchain. 

- ### [On AWS](./using-aws.md)
- ### [On Azure](./using-azure.md)
- ### [On GCP](./using-gcp.md)
- ### [Using Docker](./using-docker.md)
- ### [Using Aptos Source](./using-source-code.md)

### Configure Validator node

### Connect to Aptos network

After deploying your nodes, [connect to the Aptos Network](../connect-to-aptos-network.md).

## Set up staking and delegation pool operations

After connecting your nodes to the Aptos network, establish [staking pool operations](../staking-pool-operations.md) to add your node to the validator set. 

Similarly, conduct [delegation pool operations](../delegation-pool-operations.md) for APT delegated to your validator. Your node will start syncing and participating in consensus.

## Test Validator node

After your nodes are deployed and configure, make sure they meet [node liveness criteria](../node-liveness-criteria.md).

## Install Validator fullnode

Note that many of the same instructions can be used to run a validator fullnode in Aptos:

-  If you use the provided reference Kubernetes deployments (i.e. for cloud-managed kubernetes on AWS, Azure, or GCP), then one validator node and one validator fullnode are deployed by default.
- When using the Docker or the source code, the `fullnode.yaml` will enable you to run a validator fullnode. 
  - See [Step 11](./using-docker.md#docker-vfn) in the Docker-based instructions. 
  - Similarly, if you use source code, see from [Step 13](./using-source-code.md#source-code-vfn) in the source code instructions. 
:::