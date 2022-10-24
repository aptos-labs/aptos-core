---
title: "Running Validator Node"
slug: "running-validator-node"
---

# Running Validator Node

:::tip Deploying a validator node? Read this first
If you are deploying a validator node, then make sure to read the [Node Requirements](nodes/validator-node/operator/node-requirements.md) first.
:::


## Validator fullnode

- If you use cloud, i.e., AWS, Azure or GCP, then one validator node and one validator fullnode are deployed by default. See Step #16 in the instructions for AWS, Azure or GCP. 
- When using the Docker or the source code, the `fullnode.yaml` will enable you to run a validator fullnode. 
  - See [Step 11](nodes/validator-node/operator/running-validator-node/using-docker.md#docker-vfn) in the Docker-based instructions. 
  - Similarly, if you use source code, see from [Step 13](run-validator-node-using-source#source-code-vfn) in the source code instructions. 
:::

## Installation guides
The following guides provide step-by-step instructions for running public fullnode, validator node, and validator fullnode for the Aptos blockchain. 

- ### [On AWS](nodes/validator-node/operator/running-validator-node/using-aws.md)
- ### [On Azure](nodes/validator-node/operator/running-validator-node/using-azure.md)
- ### [On GCP](nodes/validator-node/operator/running-validator-node/using-gcp.md)
- ### [Using Docker](nodes/validator-node/operator/running-validator-node/using-docker.md)
- ### [Using Aptos Source](nodes/validator-node/operator/running-validator-node/using-source-code.md)